//! Module for handling the various checks around GCC changelogs

use std::convert::From;
use std::error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, Error as IoError, Write};
use std::process::{Command, Stdio};
use std::string::FromUtf8Error;

use crate::parser::{self, Combinator, ParseError};

#[derive(Debug)]
pub enum Error<'clog> {
    Io(IoError),
    Parser(ParseError<'clog>),
    Utf8(FromUtf8Error),
}

impl<'clog> Display for Error<'clog> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{self:#?}")
    }
}

impl<'clog> error::Error for Error<'clog> {}

impl<'clog> From<IoError> for Error<'clog> {
    fn from(e: IoError) -> Error<'clog> {
        Error::Io(e)
    }
}

impl<'clog> From<ParseError<'clog>> for Error<'clog> {
    fn from(e: ParseError<'clog>) -> Error<'clog> {
        Error::Parser(e)
    }
}

impl<'clog> From<FromUtf8Error> for Error<'clog> {
    fn from(e: FromUtf8Error) -> Error<'clog> {
        Error::Utf8(e)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Success,
    Failed,
}

#[derive(Debug)]
struct CheckLine {
    hash: String,
    status: Status,
}

// FIXME: This can return a slice of input by using indexes
fn hash(input: &str) -> Result<(&str, String), ParseError> {
    let mut hash = String::new();
    let mut input = input;

    while let Ok((new_input, c)) = parser::alphanum()(input) {
        input = new_input;
        hash.push(c);
    }

    if hash.is_empty() {
        Err(ParseError {
            input,
            combinator: Combinator::Custom("hash".to_owned()),
        })
    } else {
        Ok((input, hash))
    }
}

fn parse_checking_line(line: &str) -> Result<CheckLine, ParseError> {
    // FIXME: Add validation this is a proper hash and not some random string
    // FIXME: Should we check this hash belongs to the repository

    let (line, _) = parser::tag("Checking")(line)?;
    let (line, _) = parser::whitespace(line)?;
    let (line, hash) = hash(line)?;
    let (line, _) = parser::character(':')(line)?;
    let (line, _) = parser::whitespace(line)?;
    let (_, result) = parser::either(parser::tag("OK"), parser::tag("FAILED"))(line)?;

    Ok(CheckLine {
        hash,
        status: match result {
            "OK" => Status::Success,
            "FAILED" => Status::Failed,
            _ => unreachable!(),
        },
    })
}

pub fn check_clog_checker_output<'clog>() -> Result<(), Error<'clog>> {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        let line = line?;
        let line = parse_checking_line(&line);
        if let Ok(check_line) = line {
            if check_line.status == Status::Failed {
                let patch = Command::new("git")
                    .args(["show", &check_line.hash, "-1"])
                    .output()?
                    .stdout;
                let mut changelog_cmd =
                    Command::new("python3") /* FIXME: Is that correct? Probably not */
                        // FIXME: Fix path
                        .arg("contrib/mklog.py") /* FIXME: We should probably use a Path here */
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn()?;

                dbg!(&patch);
                dbg!(&check_line);

                changelog_cmd.stdin.take().unwrap().write_all(&patch)?;

                let cl = changelog_cmd.wait_with_output()?.stdout;
                println!(
                    "* Changelog skeleton for commit {}:\n```{}```",
                    check_line.hash,
                    String::from_utf8(cl)?
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    static _OUT: &str = r#"
Checking 71e2a04ec5668c6b1b7f5afecf6fdee4b54888fa: FAILED
ERR: cannot find a ChangeLog location in message
Checking a9422b51c1355f1414a6418e6a5ae1abdd9b9f9b: OK
Checking 9b7fe56826ef27e060d67db3f6573fea001ea477: OK
Checking 7f222689dcbd191f80bfe74a62b9c25e4215cc1e: FAILED
ERR: new file in the top-level folder not mentioned in a ChangeLog: ".github/workflows/commit-format.yml"
Checking c146eb7d99b5074bf7bb66d1a7076e138c92a808: FAILED
ERR: cannot find a ChangeLog location in message
Checking d0dad09f5de7ddcb52b825b5d3cd4f1aee16f982: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/backend/rust-compile-pattern.cc"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/backend/rust-compile-pattern.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/backend/rust-compile-stmt.cc"
Checking a731476b50301c0f551ec7964d08e669a2b13e66: FAILED
ERR: cannot find a ChangeLog location in message
Checking 026e20b1dcc4765f17d593031fe8c1114e4bfaa5: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/parse/rust-parse-impl.h"
Checking d7c321da9f8fdee4527285c782db57c7c4d520e4: FAILED
ERR: trailing whitespace: "gcc/rust/ChangeLog:
"
ERR: trailing whitespace: "
"
ERR: line should start with a tab: "
"
ERR: trailing whitespace: "	* ast/rust-macro.h (enum class): Add `BuiltinMacro` enum class
"
ERR: trailing whitespace: "	* expand/rust-attribute-visitor.cc (AttrVisitor::visit): Mention switching on `macro.kind` once builtin macro invocations are properly handled
"
ERR: line exceeds 100 character limit: "	* expand/rust-attribute-visitor.cc (AttrVisitor::visit): Mention switching on `macro.kind` once builtin macro invocations are properly handled
"
ERR: trailing whitespace: "	* parse/rust-parse-impl.h (Parser::parse_macro_invocation): Switch to new MacroInvocation API
"
ERR: line exceeds 100 character limit: "	* parse/rust-parse-impl.h (Parser::parse_macro_invocation): Switch to new MacroInvocation API
"
ERR: trailing whitespace: "	(Parser::parse_type): Switch to new MacroInvocation API
"
ERR: trailing whitespace: "	(Parser::parse_type_no_bounds): Switch to new MacroInvocation API
"
ERR: trailing whitespace: "
"
ERR: line should start with a tab: "
"
ERR: line exceeds 100 character limit: "This will be necessary for proper handling of builtin macros with the new `EarlyNameResolver` class and associated fixed-point algorithm"
ERR: line should start with a tab: "This will be necessary for proper handling of builtin macros with the new `EarlyNameResolver` class and associated fixed-point algorithm"
Checking 3630e0e2bf0fced043efb23f077adfe9b576dcad: OK
Checking 0ae13efb5a6383a3e7e22ceb064fa358038bd36f: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/ast/rust-ast.cc"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/ast/rust-expr.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/hir/tree/rust-hir-expr.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/operator.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/rust-backend.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/util/rust-lang-item.h"
Checking d17212591af78ba9894d1ca7aa457ec2340ca461: FAILED
ERR: cannot find a ChangeLog location in message
Checking e66cb001dc7a011a27c8104f9a9dd7ee6c9a6c08: FAILED
ERR: cannot find a ChangeLog location in message
Checking d12a38da686e39952e083821f1d77116f3ed91af: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/Make-lang.in"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/ast/rust-ast-full-test.cc"
Checking 2ad7c1ca0e9cff76d34b5d9acd94e2893ff538db: FAILED
ERR: cannot find a ChangeLog location in message
Checking 21ec24265b2fb5ee76f806680d30ea90b4903ec8: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/testsuite/lib/rust.exp"
Checking d4f6a97c628fe561ff5d1a14cf26113de9a869d3: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/backend/rust-compile-pattern.cc"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/backend/rust-compile-pattern.h"
Checking 2ba4506b672aa0bfadb589e66ee6983e52348d44: FAILED
ERR: cannot find a ChangeLog location in message
Checking fe828b4f931ca216a4b74322a4fdb527faada53e: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/hir/tree/rust-hir-pattern.h"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/typecheck/rust-hir-type-check-pattern.cc"
Checking 01a07f7d3959ec8bd4474a6081ebae4454c1a229: FAILED
ERR: cannot find a ChangeLog location in message
Checking 248316afc36984015b2674e15dac0c6d40c50b87: FAILED
ERR: cannot find a ChangeLog location in message
Checking a91b12e2c4cffe8386b1b712f4a5748fe81d2e8c: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/hir/rust-ast-lower-pattern.cc"
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/hir/rust-ast-lower-pattern.h"
Checking ea7893625e8246b85da4d3aa0fef40f82d619873: FAILED
ERR: cannot find a ChangeLog location in message
Checking 6a320bca7be0535b1e402ee48741df15c8e0aa8d: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/expand/rust-macro-expand.cc"
Checking 7558c183a11db3f53275c2f4691e3575a6b014e4: FAILED
ERR: changed file not mentioned in a ChangeLog: "gcc/rust/resolve/rust-ast-resolve-pattern.h"
Checking fc0c03dcf756c89694845484abd7d74ed080fdb6: FAILED
ERR: cannot find a ChangeLog location in message
Checking a194ee1dcbeae94735b279be7fa589cd5fa09bef: FAILED
ERR: cannot find a ChangeLog location in message
Checking 53c0231c78261c7d142dd71c1bc861b6298d553a: FAILED
ERR: cannot find a ChangeLog location in message
Checking 01c232573e8d9d9fac47ebfee8459e3b493ce278: FAILED
ERR: trailing whitespace: "gcc/testsuite/ChangeLog:
"
ERR: trailing whitespace: "
"
ERR: line should start with a tab: "
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_cfg.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_concat.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_env.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_include_bytes.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_include_str.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macro_line.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macros1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/builtin_macros3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/cfg1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/cfg2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/cfg3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/cfg4.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/coercion1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/coercion2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/extern_mod4.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/helloworld1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/helloworld2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1198.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1231.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1232.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1249.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1436.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-1496.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-647.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-845.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-851.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-858.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/issue-976.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros10.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros11.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros12.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros13.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros14.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros22.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros29.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros30.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros31.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros7.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros8.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/macros9.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_bool1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_byte1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_char1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_int1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_loop1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_range1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_range2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/match_tuple1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/method1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/method2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/method3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/method4.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_10.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_11.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_12.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_4.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_5.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_6.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_7.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_8.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/operator_overload_9.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/str-layout1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/str-zero.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait1.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait10.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait11.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait12.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait13.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait2.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait3.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait4.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait5.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait6.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait7.rs: Handle carriage returns properly
"
ERR: trailing whitespace: "	* rust/execute/torture/trait8.rs: Handle carriage returns properly
"
Checking 22343f79c0d4f686647cb6b220f1b1cb87c634bb: OK
Checking edc676cfe8988c62c81b0df224c7fe82583012b1: OK
Checking a75f038c069cc3a23b214854bedf04321fe88bc5: OK
Checking b07ef39ffbf4e77a586605019c64e2e070915ac3: OK
Checking 88415d33bb34c087da29938ef270788f155bb584: OK
Checking 5e7d199739f245eaceed1e74ffd48429e2401c86: OK
Checking ab1e0db43c2faf5e6dd0526a41410c36d346b4a6: OK
Checking ea34614225d4d255e58f63206eb12178b870cb4c: OK
Checking bba14a0790f0858f76118bbb9b3a8231eb241138: OK
Checking fe6264fa28a37ccbfb0e03798d1cbb11e09d1971: OK
Checking cfbda2f78baac4f329efe1838401b4ae2ed5b6a5: OK
Checking 019b2f15581948806ee14a6d05b09ec94f04c966: OK
Checking 15f04af347e3b65f436808077cbac4fa566019f9: OK
Checking 509e4c32c6a80ede6c6dda0f4cfc96f94d24c4d6: OK
Checking 4d67468d1d40f4d60a3760d47b74912c13621ada: OK
Checking 520b52b24e73d2ec48fd6f492266df42c218bdf2: OK
Checking ca246e573fb3f53fba5794f72b9245382eb46180: OK
Checking 5215235f01665062fbe182bb0c3c49539d882ad7: OK
Checking b1b35204d8a186a6fadc8534e99e9161892192ac: OK
Checking 06688fe40a249a406634d3307f662e2fe2e0c517: OK
Checking 24393cb68faadda19c9f0ba12b9bba501e8e4ff8: OK
Checking c6c3db21769e8455f38e0d6ce004c44521aad7bd: OK
Checking 9ce37e720624accb7977ead5d0f25ac2b459c2aa: OK
Checking 2e7fc8780e0d5009bc8edd116378a25dea8cb1fa: OK
Checking 9a4fee5f57c1ba6844407f81a6a40c30bc2735d4: OK
Checking eb10bc5225e03c32175b32c4778e937e64f7ddaa: OK
Checking 15b0278905ed80413867ad78868a597dd7227170: OK
Checking c7f8347e83caf8a66fb71e411415ae869c6e6a5c: OK
Checking b32b1b1576a6df965cb3fcbed3780b9f045286b2: OK
Checking 7999cf327de7b5bbea80046715eeb00c0755a08d: OK
Checking 7641eaead409ad3a80b6c92900199af352549fe4: OK
Checking 8ad1d56d68a998fdc662a944f461e7bcb125920e: OK
Checking 85a8fe00f805e7889b4e67a98ae1d435c042166b: OK
Checking 1841081a8a306c1a220694a5ddb3a927cb4b2db3: OK
Checking 32c8fb0eeafb1ec47f75242cb28171fcbdbf6e8e: OK
Checking 35e4f3b4af4c4e9a883b40603199eea09f9cd9f0: OK
Checking 18f6990f842d0bdcb2cf9541ca98d67b414d5802: OK
Checking 5b981e9c7411e68bdd73365dbd94ed3844bce2c8: OK
Checking d588754c8266d74c9eef1e38d2d96e66ff876107: OK
Checking 438ae944fa60a3d6442822cf7b41d95c47714582: OK
Checking 6b35ae12ce9371bf0ae7ad202c4393cdd77fad55: OK
Checking dc4171edb3c35690c67a928cbb431aa702bdbe79: OK
Checking 5a56869d6e339187da4a91697f1185227c8a03ba: OK
Checking 97705b4459b645770ffb6c01ff6177de6774ef3c: OK
Checking f60df7e6202300b25739b30b9e7430c0be22eb9f: OK
Checking 4b8f3005af0ddfd409f43e671b817f846e3c47e4: OK
Checking b772a504eff27c4260772752a7ad3ccaefcfc4af: OK
Checking dd950cbbb97ff5ebc203cba6c2112edd322b6f35: OK
    "#;
}
