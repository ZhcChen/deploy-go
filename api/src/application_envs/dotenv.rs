use std::collections::HashMap;

use serde::Serialize;
use utoipa::ToSchema;

pub const MAX_ENV_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct DotenvFieldError {
    pub line: usize,
    pub code: &'static str,
    pub message: &'static str,
}

pub fn validate_file_name(value: &str) -> bool {
    value.len() <= 132
        && value.ends_with(".env")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && !value.contains("..")
}

pub fn validate_module(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
}

pub fn validate(content: &str) -> Result<(), Vec<DotenvFieldError>> {
    let mut errors = Vec::new();
    if content.len() > MAX_ENV_BYTES {
        errors.push(DotenvFieldError {
            line: 1,
            code: "content_too_large",
            message: "Env 内容超过 1 MiB 上限",
        });
        return Err(errors);
    }
    if content
        .chars()
        .any(|character| character.is_control() && character != '\n')
    {
        errors.push(DotenvFieldError {
            line: 1,
            code: "control_character",
            message: "Env 内容包含不允许的控制字符",
        });
        return Err(errors);
    }

    let mut keys = HashMap::<&str, usize>::new();
    for (index, raw_line) in content.split('\n').enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with("export ") {
            errors.push(error(
                line_number,
                "export_not_supported",
                "不支持 export 语法",
            ));
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            errors.push(error(
                line_number,
                "assignment_required",
                "必须使用 KEY=VALUE 语法",
            ));
            continue;
        };
        if !valid_key(key) {
            errors.push(error(line_number, "invalid_key", "变量名格式不正确"));
            continue;
        }
        if keys.insert(key, line_number).is_some() {
            errors.push(error(line_number, "duplicate_key", "变量名重复"));
        }
        validate_value(value, line_number, &mut errors);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn valid_key(key: &str) -> bool {
    key.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn validate_value(value: &str, line: usize, errors: &mut Vec<DotenvFieldError>) {
    if value.contains('$') {
        errors.push(error(
            line,
            "expansion_not_supported",
            "不支持变量或命令展开",
        ));
    }
    let first = value.as_bytes().first().copied();
    if matches!(first, Some(b'\'' | b'\"')) {
        let quote = first.expect("已检查");
        if value.len() < 2 || value.as_bytes().last().copied() != Some(quote) {
            errors.push(error(line, "unclosed_quote", "引号未闭合"));
        } else if value.as_bytes()[1..value.len() - 1].contains(&quote) {
            errors.push(error(line, "unsupported_quote", "引号值内不支持同类引号"));
        }
    } else if value.contains(['\'', '\"']) {
        errors.push(error(line, "unsupported_quote", "引号只能包裹完整值"));
    }
}

fn error(line: usize, code: &'static str, message: &'static str) -> DotenvFieldError {
    DotenvFieldError {
        line,
        code,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_supported_document_without_rewriting_it() {
        let content = "# comment\nEMPTY=\nAPI_URL='https://example.test/a=b'\nNAME=deploy-go\n";
        assert!(validate(content).is_ok());
    }

    #[test]
    fn reports_duplicate_and_invalid_keys_by_line() {
        let errors = validate("GOOD=one\nBAD-NAME=two\nGOOD=three\n").unwrap_err();
        assert_eq!(errors[0].line, 2);
        assert_eq!(errors[0].code, "invalid_key");
        assert_eq!(errors[1].line, 3);
        assert_eq!(errors[1].code, "duplicate_key");
    }

    #[test]
    fn rejects_export_expansion_controls_and_unclosed_quotes() {
        assert!(validate("export A=1\n").is_err());
        assert!(validate("A=${OTHER}\n").is_err());
        assert!(validate("A='missing\n").is_err());
        assert!(validate("A=x\r\n").is_err());
        assert!(validate("A=$OTHER\n").is_err());
        assert!(validate("A=contains\ttab\n").is_err());
        assert!(validate("A='value' # comment\n").is_err());
        assert!(validate("A='value'junk'\n").is_err());
        assert!(validate("A=value\u{0085}next\n").is_err());
    }
}
