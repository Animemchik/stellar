use ry_ast::{Expression, IdentifierAST, Literal, Pattern, Statement};
use ry_diagnostics::GlobalDiagnostics;
use ry_filesystem::location::Location;
use ry_interner::{IdentifierInterner, DUMMY_PATH_ID};
use ry_parser::parse_statement;

#[test]
fn defer() {
    let mut identifier_interner = IdentifierInterner::new();
    let mut diagnostics = GlobalDiagnostics::new();

    assert_eq!(
        parse_statement(
            DUMMY_PATH_ID,
            "defer file.close();",
            &mut diagnostics,
            &mut identifier_interner
        ),
        Some(Statement::Defer {
            call: Expression::Call {
                location: Location {
                    file_path_id: DUMMY_PATH_ID,
                    start: 6,
                    end: 18
                },
                callee: Box::new(Expression::FieldAccess {
                    location: Location {
                        file_path_id: DUMMY_PATH_ID,
                        start: 6,
                        end: 16
                    },
                    left: Box::new(Expression::Identifier(IdentifierAST {
                        location: Location {
                            file_path_id: DUMMY_PATH_ID,
                            start: 6,
                            end: 10
                        },
                        symbol: identifier_interner.get_or_intern("file")
                    })),
                    right: IdentifierAST {
                        location: Location {
                            file_path_id: DUMMY_PATH_ID,
                            start: 11,
                            end: 16
                        },
                        symbol: identifier_interner.get_or_intern("close")
                    }
                }),
                arguments: vec![]
            }
        })
    );
}

#[test]
fn r#break() {
    let mut identifier_interner = IdentifierInterner::new();
    let mut diagnostics = GlobalDiagnostics::new();

    assert_eq!(
        parse_statement(
            DUMMY_PATH_ID,
            "break;",
            &mut diagnostics,
            &mut identifier_interner
        ),
        Some(Statement::Break {
            location: Location {
                file_path_id: DUMMY_PATH_ID,
                start: 0,
                end: 5
            }
        })
    );
}

#[test]
fn r#continue() {
    let mut identifier_interner = IdentifierInterner::new();
    let mut diagnostics = GlobalDiagnostics::new();

    assert_eq!(
        parse_statement(
            DUMMY_PATH_ID,
            "continue;",
            &mut diagnostics,
            &mut identifier_interner
        ),
        Some(Statement::Continue {
            location: Location {
                file_path_id: DUMMY_PATH_ID,
                start: 0,
                end: 8
            }
        })
    );
}

#[test]
fn r#let() {
    let mut identifier_interner = IdentifierInterner::new();
    let mut diagnostics = GlobalDiagnostics::new();

    assert_eq!(
        parse_statement(
            DUMMY_PATH_ID,
            "let x = 1;",
            &mut diagnostics,
            &mut identifier_interner
        ),
        Some(Statement::Let {
            pattern: Pattern::Identifier {
                location: Location {
                    file_path_id: DUMMY_PATH_ID,
                    start: 4,
                    end: 5
                },
                identifier: IdentifierAST {
                    location: Location {
                        file_path_id: DUMMY_PATH_ID,
                        start: 4,
                        end: 5
                    },
                    symbol: identifier_interner.get_or_intern("x")
                },
                pattern: None
            },
            value: Expression::Literal(Literal::Integer {
                value: 1,
                location: Location {
                    file_path_id: DUMMY_PATH_ID,
                    start: 8,
                    end: 9
                }
            }),
            ty: None
        })
    );
}
