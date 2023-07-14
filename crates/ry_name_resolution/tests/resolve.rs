use std::path::PathBuf;

use ry_ast::{IdentifierAst, ImportPath};
use ry_filesystem::span::DUMMY_SPAN;
use ry_fx_hash::FxHashMap;
use ry_interner::Interner;
use ry_name_resolution::{GlobalContext, ModuleContext, NameBindingData, ProjectContext};
use ry_typed_ast::Path;

/// ```txt
/// a
/// |_ a.ry
///
/// resolve(a.a) = a.a module
/// ```
#[test]
fn resolve_module() {
    let mut interner = Interner::default();
    let a = interner.get_or_intern("a");
    let b = interner.get_or_intern("b");

    let mut tree = GlobalContext::new();

    let child_module_data = ModuleContext {
        path: PathBuf::new(),
        docstring: None,
        bindings: FxHashMap::default(),
        submodules: FxHashMap::default(),
        implementations: vec![],
        imports: vec![],
    };

    let mut project_root_module = ModuleContext {
        path: PathBuf::new(),
        docstring: None,
        bindings: FxHashMap::default(),
        submodules: FxHashMap::default(),
        implementations: vec![],
        imports: vec![],
    };
    project_root_module.submodules.insert(a, child_module_data);

    let project = ProjectContext {
        path: PathBuf::new(),
        root: project_root_module,
        dependencies: vec![],
    };

    tree.projects.insert(a, project);

    assert!(matches!(
        tree.resolve_module_item_by_absolute_path(&Path {
            symbols: vec![a, a]
        }),
        Some(NameBindingData::Module(..))
    ));

    assert_eq!(
        tree.resolve_module_item_by_absolute_path(&Path { symbols: vec![a] }),
        None
    );
    assert_eq!(
        tree.resolve_module_item_by_absolute_path(&Path { symbols: vec![b] }),
        None
    );
}

/// ```txt
/// a
/// |_ project.ry
///    |_ `import a.b;`
///    |_ `import a.b as c;`
/// |_ b.ry
///
/// a/project.ry: resolve(b) = a.b, resolve(c) = a.b
/// ```
#[test]
fn import() {
    let mut interner = Interner::default();
    let a = interner.get_or_intern("a");
    let b = interner.get_or_intern("b");
    let c = interner.get_or_intern("c");

    let mut tree = GlobalContext::new();

    let child_module_data = ModuleContext {
        path: PathBuf::new(),
        docstring: None,
        bindings: FxHashMap::default(),
        submodules: FxHashMap::default(),
        implementations: vec![],
        imports: vec![],
    };

    let mut submodules = FxHashMap::default();
    submodules.insert(b, child_module_data);

    let project_root_module = ModuleContext {
        path: PathBuf::new(),
        docstring: None,
        bindings: FxHashMap::default(),
        submodules,
        implementations: vec![],
        imports: vec![
            (
                DUMMY_SPAN,
                ImportPath {
                    path: ry_ast::Path {
                        span: DUMMY_SPAN,
                        identifiers: vec![
                            IdentifierAst {
                                span: DUMMY_SPAN,
                                symbol: a,
                            },
                            IdentifierAst {
                                span: DUMMY_SPAN,
                                symbol: b,
                            },
                        ],
                    },
                    r#as: None,
                },
            ),
            (
                DUMMY_SPAN,
                ImportPath {
                    path: ry_ast::Path {
                        span: DUMMY_SPAN,
                        identifiers: vec![
                            IdentifierAst {
                                span: DUMMY_SPAN,
                                symbol: a,
                            },
                            IdentifierAst {
                                span: DUMMY_SPAN,
                                symbol: b,
                            },
                        ],
                    },
                    r#as: Some(IdentifierAst {
                        span: DUMMY_SPAN,
                        symbol: c,
                    }),
                },
            ),
        ],
    };
    let project = ProjectContext {
        path: PathBuf::new(),
        root: project_root_module,
        dependencies: vec![],
    };

    tree.projects.insert(a, project);

    assert!(matches!(
        tree.projects
            .get(&a)
            .unwrap()
            .root
            .resolve_module_item_path(&Path { symbols: vec![b] }, &tree),
        Some(..)
    ));
    assert!(matches!(
        tree.projects
            .get(&a)
            .unwrap()
            .root
            .resolve_module_item_path(&Path { symbols: vec![c] }, &tree),
        Some(..)
    ));
    assert_eq!(
        tree.projects
            .get(&a)
            .unwrap()
            .root
            .resolve_module_item_path(
                &Path {
                    symbols: vec![b, b]
                },
                &tree
            ),
        None
    );
}
