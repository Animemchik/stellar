use std::sync::Arc;

use ry_ast::{IdentifierAST, ImportPath, Visibility};
use ry_fx_hash::FxHashMap;
use ry_interner::IdentifierID;
use ry_name_resolution::{
    DefinitionID, EnumData, EnumItemID, ModuleID, ModuleScope, NameBinding, Path,
};
use ry_thir::{
    generic_parameter_scope::GenericParameterScope,
    ty::{Type, TypeConstructor},
    ModuleItemSignature,
};

use crate::{diagnostics::ExpectedType, TypeCheckingContext};

impl TypeCheckingContext<'_, '_, '_> {
    /// Adds a not analyzed module item HIR into the context.
    pub fn add_item_hir(
        &mut self,
        module_id: ModuleID,
        item: ry_hir::ModuleItem,
        imports: &mut FxHashMap<IdentifierID, NameBinding>,
        enums: &mut FxHashMap<DefinitionID, EnumData>,
    ) {
        match item {
            ry_hir::ModuleItem::Import { path, .. } => {
                self.add_import_hir(path, imports);
            }
            ry_hir::ModuleItem::Enum {
                visibility,
                name: IdentifierAST { id: name_id, .. },
                items,
                ..
            } => {
                self.add_enum_hir(module_id, visibility, name_id, items, enums);
            }
            _ => {
                let definition_id = DefinitionID {
                    name_id: item.name().unwrap(),
                    module_id,
                };

                self.resolution_environment
                    .visibilities
                    .insert(definition_id, item.visibility());
                self.hir_storage
                    .write()
                    .add_module_item(definition_id, item);
            }
        }
    }

    /// Adds an import into the context (adds it into its inner name resolution context).
    fn add_import_hir(
        &self,
        path: ry_hir::ImportPath,
        imports: &mut FxHashMap<IdentifierID, NameBinding>,
    ) {
        let ImportPath { path, r#as } = path;

        let name_id = if let Some(r#as) = r#as {
            r#as
        } else {
            *path.identifiers.last().unwrap()
        }
        .id;

        let Some(binding) = self.resolution_environment.resolve_path(
            path.clone(),
            self.identifier_interner,
            self.diagnostics,
        ) else {
            return;
        };

        imports.insert(name_id, binding);
    }

    /// Adds a not yet analyzed enum module item HIR into the context.
    fn add_enum_hir(
        &mut self,
        module_id: ModuleID,
        visibility: Visibility,
        name_id: IdentifierID,
        items: Vec<ry_hir::EnumItem>,
        enums: &mut FxHashMap<DefinitionID, EnumData>,
    ) {
        let definition_id = DefinitionID { name_id, module_id };

        let mut items_data = FxHashMap::default();

        for item in items {
            items_data.insert(
                item.symbol(),
                EnumItemID {
                    enum_definition_id: definition_id,
                    item_id: item.symbol(),
                },
            );
        }

        self.resolution_environment
            .visibilities
            .insert(definition_id, visibility);
        enums.insert(definition_id, EnumData { items: items_data });
    }

    /// Resolves all imports in the name resolution context.
    ///
    /// **WARNING**: The function must be called before any actions related to analysis or
    /// name resolution, because if not it will cause panics when trying to work with
    /// module imports.
    #[inline]
    pub fn process_imports(&mut self) {
        self.resolution_environment
            .resolve_imports(self.identifier_interner, self.diagnostics);
    }

    /// Converts a type representation from HIR into [`Type`].
    pub fn resolve_type(
        &self,
        ty: &ry_hir::Type,
        generic_parameter_scope: &GenericParameterScope,
        module_scope: &ModuleScope,
    ) -> Option<Type> {
        match ty {
            ry_hir::Type::Constructor(constructor) => self
                .resolve_type_constructor(constructor, generic_parameter_scope, module_scope)
                .map(Type::Constructor),
            ry_hir::Type::Tuple { element_types, .. } => element_types
                .into_iter()
                .map(|element| self.resolve_type(element, generic_parameter_scope, module_scope))
                .collect::<Option<Vec<_>>>()
                .map(|element_types| Type::Tuple { element_types }),
            ry_hir::Type::Function {
                parameter_types,
                return_type,
                ..
            } => Some(Type::Function {
                parameter_types: parameter_types
                    .into_iter()
                    .map(|parameter| {
                        self.resolve_type(parameter, generic_parameter_scope, module_scope)
                    })
                    .collect::<Option<_>>()?,
                return_type: Box::new(self.resolve_type(
                    return_type,
                    generic_parameter_scope,
                    module_scope,
                )?),
            }),
            ry_hir::Type::InterfaceObject { bounds, .. } => {
                let bounds = self.resolve_bounds(generic_parameter_scope, bounds, module_scope);

                if bounds.is_empty() {
                    return None;
                } else {
                    return Some(Type::InterfaceObject { bounds });
                }
            }
        }
    }

    /// Converts a type constructor from HIR into [`TypeConstructor`].
    fn resolve_type_constructor(
        &self,
        ty: &ry_hir::TypeConstructor,
        generic_parameter_scope: &GenericParameterScope,
        module_scope: &ModuleScope,
    ) -> Option<TypeConstructor> {
        let mut identifiers_iter = ty.path.identifiers.iter();
        let possible_generic_parameter_name = identifiers_iter.next().unwrap();

        if identifiers_iter.next().is_none() && ty.arguments.is_empty() {
            if generic_parameter_scope.contains(possible_generic_parameter_name.id) {
                return Some(TypeConstructor {
                    path: Path {
                        identifiers: vec![possible_generic_parameter_name.id],
                    },
                    arguments: vec![],
                });
            }
        }

        let Some(name_binding) = module_scope.resolve_path(
            ty.path.clone(),
            self.identifier_interner,
            self.diagnostics,
            &self.resolution_environment,
        ) else {
            return None;
        };

        let name_binding_kind = name_binding.kind();

        if !name_binding_kind.is_module_item() {
            self.diagnostics.write().add_single_file_diagnostic(
                ty.location.file_path_id,
                ExpectedType {
                    location: ty.location,
                    name_binding_kind,
                },
            );

            return None;
        }

        todo!()
    }

    /// Resolves type arguments.
    fn resolve_type_arguments(
        &self,
        hir: &[ry_hir::Type],
        generic_parameter_scope: &GenericParameterScope,
        module_scope: &ModuleScope,
    ) -> Option<Vec<Type>> {
        hir.into_iter()
            .map(|ty| self.resolve_type(ty, generic_parameter_scope, module_scope))
            .collect::<Option<_>>()
    }

    fn unwrap_type_alias(&self, path: Path) -> Type {
        let definition_id = self.resolve_type_signature_by_path(path);
        todo!()
    }

    fn implements(&self, ty: Type, interface: TypeConstructor) -> bool {
        match ty {
            Type::Constructor(constructor) => {
                let signature = self.resolve_type_signature_by_path(constructor.path);

                match signature.as_ref() {
                    ModuleItemSignature::TypeAlias(alias) => {}
                    _ => {}
                }

                todo!()
            }
            _ => false, // implement builtin interfaces later
        }
    }

    pub(crate) fn resolve_interface(
        &self,
        interface: ry_hir::TypeConstructor,
        generic_parameter_scope: &GenericParameterScope,
        module_scope: &ModuleScope,
    ) -> Option<TypeConstructor> {
        let Some(name_binding) = module_scope.resolve_path(
            interface.path.clone(),
            self.identifier_interner,
            self.diagnostics,
            &self.resolution_environment,
        ) else {
            return None;
        };

        let signature = self.resolve_signature(name_binding, module_scope)?;

        match signature.as_ref() {
            ModuleItemSignature::Interface(_) => Some(TypeConstructor {
                path: Path {
                    identifiers: interface
                        .path
                        .identifiers
                        .iter()
                        .map(|identifier| identifier.id)
                        .collect(),
                },
                arguments: self.resolve_type_arguments(
                    &interface.arguments,
                    generic_parameter_scope,
                    module_scope,
                )?,
            }),
            _ => unreachable!(),
        }
    }

    pub(crate) fn resolve_bounds(
        &self,
        generic_parameter_scope: &GenericParameterScope,
        bounds: &[ry_hir::TypeConstructor],
        module_scope: &ModuleScope,
    ) -> Vec<TypeConstructor> {
        bounds
            .into_iter()
            .filter_map(|bound| {
                self.resolve_interface(bound.clone(), generic_parameter_scope, module_scope)
            })
            .collect()
    }

    fn resolve_type_signature_by_definition_id(
        &self,
        definition_id: DefinitionID,
    ) -> Arc<ModuleItemSignature> {
        todo!()
    }

    fn resolve_type_signature_by_path(&self, path: Path) -> Arc<ModuleItemSignature> {
        todo!()
    }

    fn resolve_interface_signature_by_definition_id(
        &self,
        definition_id: DefinitionID,
    ) -> Arc<ModuleItemSignature> {
        todo!()
    }

    fn resolve_interface_signature_by_path(&self, path: Path) -> Arc<ModuleItemSignature> {
        todo!()
    }
}
