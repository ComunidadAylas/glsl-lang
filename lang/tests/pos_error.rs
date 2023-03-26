use expect_test::{expect, Expect};
use glsl_lang::parse::{IntoParseBuilderExt, ParseOptions};
use lang_util::FileId;

fn check<E: std::error::Error>(src: E, expected: Expect) {
    let actual = src.to_string();
    expected.assert_eq(&actual);
}

#[test]
fn str_pos_error() {
    let tu: Result<glsl_lang::ast::TranslationUnit, _> =
        include_str!("../data/tests/pos_error_b.glsl")
            .builder()
            .opts(&ParseOptions {
                source_id: FileId::new(10),
                ..Default::default()
            })
            .parse()
            .map(|(tu, _, _)| tu);

    check(
        tu.unwrap_err(),
        expect![[r##"10:1:1: '#error' : This error should be in pos_error_b.glsl"##]],
    );
}
