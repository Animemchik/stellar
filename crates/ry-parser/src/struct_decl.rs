use crate::{error::ParserError, macros::*, Parser, ParserResult};
use ry_ast::{location::Span, token::RawToken::*, *};

impl<'c> Parser<'c> {
    pub(crate) fn parse_struct_declaration(&mut self, public: Option<Span>) -> ParserResult<Item> {
        self.advance(false)?; // `struct`

        check_token!(self, Identifier => "struct name in struct declaration")?;

        let name = self.current_ident_with_span();

        self.advance(false)?; // name

        let generic_annotations = self.parse_generic_annotations()?;

        check_token!(self, OpenBrace => "struct declaration")?;

        self.advance(true)?; // `{`

        let members = self.parse_struct_members()?;

        check_token!(self, CloseBrace => "struct declaration")?;

        self.advance(true)?; // `}`

        Ok(Item::StructDecl(StructDecl {
            generic_annotations,
            public,
            name,
            members,
        }))
    }

    fn parse_struct_member(&mut self) -> ParserResult<StructMemberDef> {
        let mut public = None;
        let mut r#mut = None;

        if self.current.value.is(Mut) {
            r#mut = Some(self.current.span);
            self.advance(false)?;
        }

        if self.current.value.is(Pub) {
            public = Some(self.current.span);
            self.advance(false)?;
        }

        if self.current.value.is(Mut) {
            r#mut = Some(self.current.span);
            self.advance(false)?;
        }

        check_token!(self, Identifier => "struct member name in struct definition")?;

        let name = self.current_ident_with_span();

        self.advance(false)?;

        let r#type = self.parse_type()?;

        check_token!(self, Semicolon => "struct member definition")?;

        self.advance(true)?; // `;`

        Ok(StructMemberDef {
            public,
            r#mut,
            name,
            r#type,
        })
    }

    fn parse_struct_members(&mut self) -> ParserResult<Vec<(Docstring, StructMemberDef)>> {
        let mut members = vec![];

        while !self.current.value.is(CloseBrace) {
            members.push((self.consume_local_docstring()?, self.parse_struct_member()?));
        }

        Ok(members)
    }
}

#[cfg(test)]
mod struct_tests {
    use crate::{macros::parser_test, Parser};
    use string_interner::StringInterner;

    parser_test!(empty_struct, "struct test {}");
    parser_test!(
        r#struct,
        "struct test[T, M] { pub mut a i32; mut pub b T; pub c T; d M; }"
    );
}
