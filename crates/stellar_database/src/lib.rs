#![allow(warnings)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/quantumatic/stellar/main/additional/icon/stellar.png",
    html_favicon_url = "https://raw.githubusercontent.com/quantumatic/stellar/main/additional/icon/stellar.png"
)]

use derive_more::Display;
use parking_lot::{RawRwLock, RwLock, RwLockReadGuard, RwLockWriteGuard};
use paste::paste;
use stellar_ast::{IdentifierAST, Path, Visibility};
use stellar_diagnostics::Diagnostics;
use stellar_filesystem::location::{Location, DUMMY_LOCATION};
use stellar_fx_hash::FxHashMap;
use stellar_interner::{IdentifierID, PathID};
use stellar_thir::ty::{Type, TypeConstructor};

macro_rules! define_symbol_struct {
    ($($name:ident),*) => {
        paste! {
            /// A symbol's unique ID.
            #[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
            pub enum Symbol {
                $(
                    #[doc = "A " $name "."]
                    [<$name:camel>]([<$name:camel ID>]),
                )*
            }

            impl Symbol {
                $(
                    #[doc = "Returns `true` if the symbol is a " $name "."]
                    #[doc = ""]
                    #[doc = "_This function is automatically generated by a macro._"]
                    #[inline(always)]
                    #[must_use]
                    pub const fn [<is_ $name>](&self) -> bool {
                        matches!(self, Self::[<$name:camel>](_))
                    }

                    #[doc = "Returns " $name " ID if the symbol is a " $name "."]
                    #[doc = ""]
                    #[doc = "_This function is automatically generated by a macro._"]
                    #[inline(always)]
                    #[must_use]
                    pub fn [<to_ $name>](self) -> Option<[<$name:camel ID>]> {
                        match self {
                            Self::[<$name:camel>](id) => Some(id),
                            _ => None
                        }
                    }

                    #[doc = "Returns " $name " ID if the symbol is a " $name "."]
                    #[doc = "# Panics"]
                    #[doc = "Panics if the symbol is not a " $name "."]
                    #[doc = ""]
                    #[doc = "_This function is automatically generated by a macro._"]
                    #[inline(always)]
                    #[must_use]
                    pub fn [<to_ $name _or_panic>](self) -> [<$name:camel ID>] {
                        self.[<to_ $name>]().unwrap()
                    }
                )*
            }
        }
    };
}

define_symbol_struct!(
    module,
    enum,
    struct,
    function,
    interface,
    tuple_like_struct,
    type_alias,
    enum_item
);

impl Symbol {
    /// Returns the name of the symbol.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        match self {
            Self::Module(module) => IdentifierAST {
                location: DUMMY_LOCATION,
                id: db.module(module).name,
            },
            Self::Enum(enum_) => enum_.name(db),
            Self::Struct(struct_) => struct_.name(db),
            Self::Function(function) => function.name(db),
            Self::Interface(interface) => interface.name(db),
            Self::TupleLikeStruct(struct_) => struct_.name(db),
            Self::TypeAlias(alias) => alias.name(db),
            Self::EnumItem(item) => item.name(db),
        }
    }
}

/// A data that Stellar compiler has about an enum.
#[derive(Debug)]
pub struct EnumData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub module: ModuleID,
    pub implements: Vec<TypeConstructor>,
    pub predicates: Vec<PredicateID>,
    pub items: FxHashMap<IdentifierID, EnumItemID>,
    pub methods: FxHashMap<IdentifierID, FunctionID>,
}

impl EnumData {
    /// Creates a new enum data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        module: ModuleID,
    ) -> EnumID {
        db.add_enum_module_item(Self::new(visibility, name, module))
    }

    /// Creates a new enum data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, module: ModuleID) -> Self {
        Self {
            visibility,
            name,
            module,
            implements: Vec::new(),
            predicates: Vec::new(),
            items: FxHashMap::default(),
            methods: FxHashMap::default(),
        }
    }
}

/// A unique ID that maps to [`EnumData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct EnumID(pub usize);

impl EnumID {
    /// Returns the name of the enum.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.enum_module_item(self).name
    }

    /// Returns the module which enum is defined in.
    #[inline(always)]
    #[must_use]
    pub fn module(self, db: &Database) -> ModuleID {
        db.enum_module_item(self).module
    }

    /// Returns a list of interfaces implemented by the enum.
    #[inline(always)]
    #[must_use]
    pub fn implements(self, db: &Database) -> &[TypeConstructor] {
        &db.enum_module_item(self).implements
    }

    /// Returns a list of predicates associated with the enum.
    #[inline(always)]
    #[must_use]
    pub fn predicates(self, db: &Database) -> &[PredicateID] {
        &db.enum_module_item(self).predicates
    }

    /// Returns a list of items associated with the enum.
    #[inline(always)]
    #[must_use]
    pub fn items(self, db: &Database) -> &FxHashMap<IdentifierID, EnumItemID> {
        &db.enum_module_item(self).items
    }

    /// Returns `true` if an item with a given name is contained in the enum definition.
    #[inline(always)]
    #[must_use]
    pub fn contains_item(self, db: &Database, name: IdentifierID) -> bool {
        db.enum_module_item(self).items.contains_key(&name)
    }

    /// Returns an item with a given name.
    pub fn item(self, db: &Database, name: IdentifierID) -> Option<EnumItemID> {
        db.enum_module_item(self).items.get(&name).copied()
    }
}

/// A data that Stellar compiler has about a struct.
#[derive(Debug)]
pub struct StructData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub module: ModuleID,
    pub predicates: Vec<PredicateID>,
    pub fields: FxHashMap<IdentifierID, FieldID>,
    pub methods: FxHashMap<IdentifierID, FunctionID>,
}

impl StructData {
    /// Creates a new struct data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        module: ModuleID,
    ) -> StructID {
        db.add_struct_module_item(Self::new(visibility, name, module))
    }

    /// Creates a new struct data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, module: ModuleID) -> Self {
        Self {
            visibility,
            name,
            module,
            predicates: Vec::new(),
            fields: FxHashMap::default(),
            methods: FxHashMap::default(),
        }
    }
}

/// A unique ID that maps to [`StructData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct StructID(pub usize);

impl StructID {
    /// Returns the name of the struct.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.struct_module_item(self).name
    }

    /// Returns the module which struct is defined in.
    #[inline(always)]
    #[must_use]
    pub fn module(self, db: &Database) -> ModuleID {
        db.struct_module_item(self).module
    }

    /// Returns a list of predicates associated with the struct.
    #[inline(always)]
    #[must_use]
    pub fn predicates(self, db: &Database) -> &[PredicateID] {
        &db.struct_module_item(self).predicates
    }

    /// Returns a list of fields associated with the struct.
    #[inline(always)]
    #[must_use]
    pub fn fields(self, db: &Database) -> &FxHashMap<IdentifierID, FieldID> {
        &db.struct_module_item(self).fields
    }
}

/// A data that Stellar compiler has about a function.
#[derive(Debug)]
pub struct TupleLikeStructData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub fields: Vec<(Visibility, Type)>,
    pub module: ModuleID,
}

impl TupleLikeStructData {
    /// Creates a new tuple-like struct data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        module: ModuleID,
    ) -> TupleLikeStructID {
        db.add_tuple_like_struct(Self::new(visibility, name, module))
    }

    /// Creates a new tuple-like struct data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, module: ModuleID) -> Self {
        Self {
            visibility,
            name,
            fields: Vec::new(),
            module,
        }
    }
}

/// A unique ID that maps to [`TupleLikeStructData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TupleLikeStructID(pub usize);

impl TupleLikeStructID {
    /// Returns the name of the struct.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.tuple_like_struct(self).name
    }
}

/// A data that Stellar compiler has about a field.
#[derive(Debug)]
pub struct FieldData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub ty: Type,
}

impl FieldData {
    /// Creates a new field data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        ty: Type,
    ) -> FieldID {
        db.add_field(Self::new(visibility, name, ty))
    }

    /// Creates a new field data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, ty: Type) -> Self {
        Self {
            visibility,
            name,
            ty,
        }
    }
}

/// A unique ID that maps to [`FieldData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FieldID(pub usize);

/// A data that Stellar compiler has about a predicate.
#[derive(Debug)]
pub struct PredicateData {
    pub ty: Type,
    pub bounds: Vec<TypeConstructor>,
}

impl PredicateData {
    /// Creates a new predicate data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(db: &mut Database, ty: Type, bounds: Vec<TypeConstructor>) -> PredicateID {
        db.add_predicate(Self::new(ty, bounds))
    }

    /// Creates a new predicate data object.
    #[inline(always)]
    #[must_use]
    pub fn new(ty: Type, bounds: Vec<TypeConstructor>) -> Self {
        Self { ty, bounds }
    }
}

/// A unique ID that maps to [`PredicateData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct PredicateID(pub usize);

/// A data that Stellar compiler has about a generic parameter scope.
#[derive(Default, PartialEq, Clone, Debug)]
pub struct GenericParameterScopeData {
    /// A parent scope, for example:
    ///
    /// ```stellar
    /// interface Foo[T] { // self.parent = Scope { parent: None, parameters: [T] }
    ///     fun bar[M]();  // self = Scope { parent: ..., parameters: [M] }
    /// }
    /// ```
    pub parent_scope: Option<GenericParameterScopeID>,

    /// A map of generic parameters in the scope.
    pub parameters: FxHashMap<IdentifierID, GenericParameterID>,
}

impl GenericParameterScopeData {
    /// Creates a new empty generic parameter scope.
    #[inline(always)]
    #[must_use]
    pub fn new(parent_scope: Option<GenericParameterScopeID>) -> Self {
        Self {
            parent_scope,
            parameters: FxHashMap::default(),
        }
    }
}

/// A unique ID that maps to [`GenericParameterScopeData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericParameterScopeID(pub usize);

impl GenericParameterScopeID {
    /// Returns the parent scope.
    pub fn parent_scope(self, db: &Database) -> Option<GenericParameterScopeID> {
        db.generic_parameter_scope(self).parent_scope
    }

    /// Returns the map of generic parameters in the scope.
    pub fn parameters(self, db: &Database) -> &FxHashMap<IdentifierID, GenericParameterID> {
        &db.generic_parameter_scope(self).parameters
    }

    /// Adds a generic parameter into the scope.
    #[inline(always)]
    pub fn add_generic_parameter(
        self,
        db: &mut Database,
        parameter_name: IdentifierID,
        parameter: GenericParameterID,
    ) {
        db.generic_parameter_scope_mut(self)
            .parameters
            .insert(parameter_name, parameter);
    }

    /// Resolves a data about generic parameter in the scope.
    ///
    /// **Note**: the method shouldn't be used to check if the parameter exists
    /// in the scope. Use the [`contains()`] method.
    ///
    /// [`contains()`]: GenericParameterScopeID::contains
    #[inline(always)]
    #[must_use]
    pub fn resolve(
        &self,
        db: &Database,
        parameter_name: IdentifierID,
    ) -> Option<GenericParameterID> {
        if let Some(parameter_id) = self.parameters(db).get(&parameter_name) {
            Some(*parameter_id)
        } else if let Some(parent_scope_id) = &self.parent_scope(db) {
            parent_scope_id.resolve(db, parameter_name)
        } else {
            None
        }
    }

    /// Checks if the generic parameter exists in the scope.
    #[inline(always)]
    #[must_use]
    pub fn contains(&self, db: &Database, parameter_name: IdentifierID) -> bool {
        self.parameters(db).contains_key(&parameter_name)
            || if let Some(parent_scope_id) = &self.parent_scope(db) {
                parent_scope_id.contains(db, parameter_name)
            } else {
                false
            }
    }
}

/// A data, that the Stellar compiler has about a generic parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericParameterData {
    /// Location of the name of the generic parameter.
    ///
    /// ```txt
    /// foo[T: ToString = String]
    ///     ^
    /// ```
    pub location: Location,

    /// Default value of the generic parameter.
    ///
    /// ```txt
    /// foo[T: ToString = String]
    ///                   ^^^^^^
    /// ```
    pub default_value: Option<Type>,
}

/// A unique ID that maps to [`GenericParameterData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GenericParameterID(pub usize);

/// A data that Stellar compiler has about an enum item.
#[derive(Debug)]
pub struct EnumItemData {
    pub name: IdentifierAST,
    pub module: ModuleID,
}

impl EnumItemData {
    /// Creates a new enum item data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(db: &mut Database, name: IdentifierAST, module: ModuleID) -> EnumItemID {
        db.add_enum_item(Self::new(name, module))
    }

    /// Creates a new enum item data object.
    #[inline(always)]
    #[must_use]
    pub fn new(name: IdentifierAST, module: ModuleID) -> Self {
        Self { name, module }
    }
}

/// A unique ID that maps to [`EnumItemData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct EnumItemID(pub usize);

impl EnumItemID {
    /// Returns the name of the enum item.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.enum_item(self).name
    }

    #[inline(always)]
    #[must_use]
    pub fn module(self, db: &Database) -> ModuleID {
        db.enum_item(self).module
    }
}

/// A data that Stellar compiler has about a function.
#[derive(Debug)]
pub struct FunctionData {
    pub name: IdentifierAST,
    pub visibility: Visibility,
    pub module: ModuleID,
}

impl FunctionData {
    /// Creates a new function data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        name: IdentifierAST,
        visibility: Visibility,
        module: ModuleID,
    ) -> FunctionID {
        db.add_function(Self::new(name, visibility, module))
    }

    /// Creates a new function data object.
    #[inline(always)]
    #[must_use]
    pub fn new(name: IdentifierAST, visibility: Visibility, module: ModuleID) -> Self {
        Self {
            name,
            visibility,
            module,
        }
    }
}

/// A unique ID that maps to [`FunctionData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct FunctionID(pub usize);

impl FunctionID {
    /// Returns the name of the function.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.function(self).name
    }
}

/// A data that Stellar compiler has about an interface.
#[derive(Debug)]
pub struct InterfaceData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub module: ModuleID,
    pub predicates: Vec<PredicateID>,
    pub methods: FxHashMap<IdentifierID, FunctionID>,
}

impl InterfaceData {
    /// Creates a new interface data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        module: ModuleID,
    ) -> InterfaceID {
        db.add_interface(Self::new(visibility, name, module))
    }

    /// Creates a new interface data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, module: ModuleID) -> Self {
        Self {
            visibility,
            name,
            module,
            predicates: Vec::new(),
            methods: FxHashMap::default(),
        }
    }
}

/// A unique ID that maps to [`InterfaceData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct InterfaceID(pub usize);

impl InterfaceID {
    /// Returns the name of the interface.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.interface(self).name
    }
}

/// A data that Stellar compiler has about a module.
#[derive(Debug)]
pub struct TypeAliasData {
    pub visibility: Visibility,
    pub name: IdentifierAST,
    pub ty: Type,
    pub module: ModuleID,
}

impl TypeAliasData {
    /// Creates a new type alias data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(
        db: &mut Database,
        visibility: Visibility,
        name: IdentifierAST,
        module: ModuleID,
    ) -> TypeAliasID {
        db.add_type_alias(Self::new(visibility, name, module))
    }

    /// Creates a new type alias data object.
    #[inline(always)]
    #[must_use]
    pub fn new(visibility: Visibility, name: IdentifierAST, module: ModuleID) -> Self {
        Self {
            visibility,
            name,
            ty: Type::Unknown,
            module,
        }
    }
}

/// A unique ID that maps to [`TypeAliasData`].
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct TypeAliasID(pub usize);

impl TypeAliasID {
    /// Returns the name of the type alias.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierAST {
        db.type_alias(self).name
    }
}

/// A data that Stellar compiler has about a module.
#[derive(Debug)]
pub struct ModuleData {
    pub name: IdentifierID,
    pub filepath: PathID,
    pub module_item_symbols: FxHashMap<IdentifierID, Symbol>,
    pub submodules: FxHashMap<IdentifierID, ModuleID>,
    pub resolved_imports: FxHashMap<IdentifierID, Symbol>,
}

impl ModuleData {
    /// Creates a new module data object in the database and returns its ID.
    #[inline(always)]
    #[must_use]
    pub fn alloc(db: &mut Database, name: IdentifierID, filepath: PathID) -> ModuleID {
        db.add_module(Self::new(name, filepath))
    }

    /// Creates a new module data object.
    #[inline(always)]
    #[must_use]
    pub fn new(name: IdentifierID, filepath: PathID) -> Self {
        Self {
            name,
            filepath,
            submodules: FxHashMap::default(),
            resolved_imports: FxHashMap::default(),
            module_item_symbols: FxHashMap::default(),
        }
    }
}

/// A unique ID that maps to [`ModuleData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Display)]
pub struct ModuleID(pub usize);

impl ModuleID {
    /// Returns module's file path ID.
    #[inline(always)]
    #[must_use]
    pub fn filepath(self, db: &Database) -> PathID {
        db.module(self).filepath
    }

    /// Returns module's name.
    #[inline(always)]
    #[must_use]
    pub fn name(self, db: &Database) -> IdentifierID {
        db.module(self).name
    }

    /// Returns an immutable reference to module item symbols.
    #[inline(always)]
    #[must_use]
    pub fn module_item_symbols(self, db: &Database) -> &FxHashMap<IdentifierID, Symbol> {
        &db.module(self).module_item_symbols
    }

    /// Returns a mutable reference to module item symbols.
    #[inline(always)]
    #[must_use]
    pub fn module_item_symbols_mut(
        self,
        db: &mut Database,
    ) -> &mut FxHashMap<IdentifierID, Symbol> {
        &mut db.module_mut(self).module_item_symbols
    }

    /// Returns an immutable reference to submodules.
    #[inline(always)]
    #[must_use]
    pub fn submodules(self, db: &Database) -> &FxHashMap<IdentifierID, ModuleID> {
        &db.module(self).submodules
    }

    /// Returns a mutable reference to submodules.
    #[inline(always)]
    #[must_use]
    pub fn submodules_mut(self, db: &mut Database) -> &mut FxHashMap<IdentifierID, ModuleID> {
        &mut db.module_mut(self).submodules
    }

    /// Resolves a symbol related to only module item in the module.
    ///
    /// If you want to additionally resolve submodules, use [`ModuleData::get_symbol()`].
    #[inline(always)]
    pub fn module_item_symbol(self, db: &Database, item_name: IdentifierID) -> Option<Symbol> {
        self.module_item_symbols(db).get(&item_name).copied()
    }

    /// Resolves a symbol in the module.
    #[inline(always)]
    pub fn symbol(self, db: &Database, name: IdentifierID) -> Option<Symbol> {
        self.module_item_symbol(db, name)
            .or(self.submodule(db, name).map(Symbol::Module))
    }

    /// Resolves a symbol in the module.
    ///
    /// # Panics
    /// Panics if the symbol cannot be resolved.
    #[inline(always)]
    #[must_use]
    pub fn symbol_or_panic(self, db: &Database, name: IdentifierID) -> Symbol {
        self.symbol(db, name).unwrap()
    }

    /// Resolves a symbol in the module.
    ///
    /// # Panics
    /// Panics if the symbol cannot be resolved.
    #[inline(always)]
    #[must_use]
    pub fn module_item_symbol_or_panic(self, db: &Database, name: IdentifierID) -> Symbol {
        self.module_item_symbol(db, name).unwrap()
    }

    /// Adds a module item information to the module.
    #[inline(always)]
    pub fn add_module_item(self, db: &mut Database, name: IdentifierID, symbol: Symbol) {
        self.module_item_symbols_mut(db).insert(name, symbol);
    }

    /// Checks if a symbol is contained in the module.
    #[inline(always)]
    #[must_use]
    pub fn contains_module_item_symbol(self, db: &Database, item_name: IdentifierID) -> bool {
        self.module_item_symbols(db).contains_key(&item_name)
    }

    /// Returns an ID of the submodule of the module by its name.
    #[inline(always)]
    pub fn submodule(self, db: &Database, name: IdentifierID) -> Option<ModuleID> {
        self.submodules(db).get(&name).copied()
    }

    /// Adds a submodule to the module.
    #[inline(always)]
    pub fn add_submodule(self, db: &mut Database, module: ModuleID) {
        let name = module.name(&db);

        self.submodules_mut(db).insert(name, module);
    }

    /// Checks if a submodule with a given name is contained in the module.
    #[inline(always)]
    #[must_use]
    pub fn contains_submodule_with_name(self, db: &Database, name: IdentifierID) -> bool {
        self.submodules(db).contains_key(&name)
    }

    /// Checks if a submodule with a given ID is contained in the module.
    #[inline(always)]
    #[must_use]
    pub fn contains_submodule_with_id(self, db: &Database, id: ModuleID) -> bool {
        self.submodules(db)
            .values()
            .any(|&submodule| submodule == id)
    }

    /// Returns an immutable reference to imports.
    #[inline(always)]
    #[must_use]
    pub fn resolved_imports(self, db: &Database) -> &FxHashMap<IdentifierID, Symbol> {
        &db.module(self).resolved_imports
    }

    /// Returns a mutable reference to imports.
    #[inline(always)]
    #[must_use]
    pub fn resolved_imports_mut(self, db: &mut Database) -> &mut FxHashMap<IdentifierID, Symbol> {
        &mut db.module_mut(self).resolved_imports
    }

    /// Adds a resolved import to the module.
    #[inline(always)]
    pub fn add_resolved_import(self, db: &mut Database, name: IdentifierID, symbol: Symbol) {
        self.resolved_imports_mut(db).insert(name, symbol);
    }
}

/// Storage for Stellar compiler entities.
#[derive(Default, Debug)]
pub struct Database {
    packages: FxHashMap<IdentifierID, ModuleID>,
    modules: Vec<ModuleData>,
    enums: Vec<EnumData>,
    enum_items: Vec<EnumItemData>,
    predicates: Vec<PredicateData>,
    structs: Vec<StructData>,
    tuple_like_structs: Vec<TupleLikeStructData>,
    fields: Vec<FieldData>,
    functions: Vec<FunctionData>,
    interfaces: Vec<InterfaceData>,
    type_aliases: Vec<TypeAliasData>,
    generic_parameter_scopes: Vec<GenericParameterScopeData>,
    generic_parameters: Vec<GenericParameterData>,
}

macro_rules! db_methods {
    (
        $($what:ident($whats:ident): $id_ty:ty => $data_ty:ty),*
    ) => {
        $(
            paste! {
                #[doc = "Returns an immutable reference to " $what " data by its ID."]
                #[doc = "# Panics"]
                #[doc = "Panics if " $what " with the given ID is not present in the database storage."]
                #[doc = ""]
                #[doc = "_This function is automatically generated using a macro!_"]
                #[inline(always)]
                #[must_use]
                pub fn $what(&self, id: $id_ty) -> &$data_ty {
                    &self.$whats[id.0]
                }

                #[doc = "Returns a mutable reference to " $what " data by its ID."]
                #[doc = "# Panics"]
                #[doc = "Panics if " $what " with the given ID is not present in the database storage."]
                #[doc = ""]
                #[doc = "_This function is automatically generated using a macro!_"]
                #[inline(always)]
                #[must_use]
                pub fn [<$what _mut>](&mut self, id: $id_ty) -> &mut $data_ty {
                    &mut self.$whats[id.0]
                }

                #[doc = "Returns whether " $what " with a given ID is present in the database storage."]
                #[doc = ""]
                #[doc = "_This function is automatically generated using a macro!_"]
                #[inline(always)]
                #[must_use]
                pub fn [<contains_ $what>](&self, id: $id_ty) -> bool {
                    id.0 < self.$whats.len()
                }

                #[doc = "Adds a " $what " to the database storage."]
                #[doc = ""]
                #[doc = "_This function is automatically generated using a macro!_"]
                #[inline(always)]
                #[must_use]
                pub fn [<add_ $what>](&mut self, [<$what _>]: $data_ty) -> $id_ty {
                    self.$whats.push([<$what _>]);

                    $id_ty(self.$whats.len() - 1)
                }
            }
        )*
    };
}

impl Database {
    /// Creates a new empty database.
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // Returns a package's root module ID data by package ID.
    #[inline(always)]
    pub fn package_root_module(&self, package_name: IdentifierID) -> Option<ModuleID> {
        self.packages.get(&package_name).copied()
    }

    /// Returns a package's root module ID data by package ID.
    /// # Panics
    /// Panics if the package information is not present in the database storage.
    #[inline(always)]
    #[must_use]
    pub fn package_root_module_or_panic(&self, package_name: IdentifierID) -> ModuleID {
        *self.packages.get(&package_name).unwrap()
    }

    /// Returns wether a package with a given name is present in the database storage.
    #[inline(always)]
    #[must_use]
    pub fn contains_package(&self, package_name: IdentifierID) -> bool {
        self.packages.contains_key(&package_name)
    }

    /// Adds a package to the database storage.
    #[inline(always)]
    pub fn add_package(&mut self, root_module: ModuleID) {
        let name = root_module.name(self);
        self.packages.insert(name, root_module);
    }

    // reduces the size of code in hundreds of times!
    db_methods! {
        module(modules):            ModuleID => ModuleData,
        enum_module_item(enums):
                                    EnumID => EnumData,
        struct_module_item(structs):
                                    StructID => StructData,
        tuple_like_struct(tuple_like_structs):
                                    TupleLikeStructID => TupleLikeStructData,
        type_alias(type_aliases):   TypeAliasID => TypeAliasData,
        function(functions):        FunctionID => FunctionData,
        interface(interfaces):      InterfaceID => InterfaceData,
        predicate(predicates):      PredicateID => PredicateData,
        enum_item(enum_items):      EnumItemID => EnumItemData,
        field(fields):              FieldID =>   FieldData,
        generic_parameter_scope(generic_parameter_scopes):
                                    GenericParameterScopeID => GenericParameterScopeData,
        generic_parameter(generic_parameters):
                                    GenericParameterID => GenericParameterData
    }
}

/// Contains database and diagnostics.
#[derive(Default)]
pub struct State {
    db: Database,
    diagnostics: Diagnostics,
    config: Config,
}

pub struct Config {}

impl Default for Config {
    #[inline(always)]
    fn default() -> Self {
        Self {}
    }
}

impl Config {
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl State {
    /// Creates a new empty state.
    #[inline(always)]
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds a new state with given configuration.
    #[inline(always)]
    #[must_use]
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Returns a reference to config.
    #[inline(always)]
    #[must_use]
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Returns an immutable reference to a database object.
    #[inline(always)]
    #[must_use]
    pub const fn db(&self) -> &Database {
        &self.db
    }

    /// Returns a mutable reference to a database object.
    #[inline(always)]
    #[must_use]
    pub fn db_mut(&mut self) -> &mut Database {
        &mut self.db
    }

    /// Gives an ownership over database object inside the state.
    #[inline(always)]
    #[must_use]
    pub fn into_db(self) -> Database {
        self.db
    }

    /// Returns an immutable reference to diagnostics.
    #[inline(always)]
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Returns a mutable reference to diagnostics.
    #[inline(always)]
    #[must_use]
    pub fn diagnostics_mut(&mut self) -> &mut Diagnostics {
        &mut self.diagnostics
    }

    /// Gives an ownership over diagnostics object inside the state.
    #[inline(always)]
    #[must_use]
    pub fn into_diagnostics(self) -> Diagnostics {
        self.diagnostics
    }
}
