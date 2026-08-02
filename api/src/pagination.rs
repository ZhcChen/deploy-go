use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListQuery {
    pub limit: Option<u32>,
    pub after: Option<String>,
}

pub(crate) fn limit(query: &ListQuery, request_id: &str) -> ApiResult<usize> {
    let limit = query.limit.unwrap_or(50);
    if !(1..=200).contains(&limit) {
        return Err(ApiError::validation("limit 必须介于 1 和 200", request_id));
    }
    Ok(limit as usize)
}

pub(crate) fn decode_after(
    query: &ListQuery,
    request_id: &str,
) -> ApiResult<Option<(String, String)>> {
    query
        .after
        .as_deref()
        .map(decode)
        .transpose()
        .map_err(|_| ApiError::validation("列表游标格式不正确", request_id))
}

pub(crate) fn encode(first: &str, second: &str) -> String {
    URL_SAFE_NO_PAD.encode(format!("{first}\0{second}"))
}

fn decode(value: &str) -> Result<(String, String), ()> {
    let decoded = URL_SAFE_NO_PAD.decode(value).map_err(|_| ())?;
    let decoded = String::from_utf8(decoded).map_err(|_| ())?;
    let (first, second) = decoded.split_once('\0').ok_or(())?;
    if first.is_empty() || second.is_empty() || second.contains('\0') {
        return Err(());
    }
    Ok((first.to_owned(), second.to_owned()))
}

pub(crate) fn finish<T, F>(mut rows: Vec<T>, limit: usize, key: F) -> (Vec<T>, Option<String>)
where
    F: Fn(&T) -> (&str, &str),
{
    let has_more = rows.len() > limit;
    rows.truncate(limit);
    let next = has_more.then(|| {
        let (first, second) = key(rows.last().expect("有下一页时当前页不能为空"));
        encode(first, second)
    });
    (rows, next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_round_trip_and_rejects_invalid_values() {
        let cursor = encode("2026-08-02T00:00:00Z", "app_01");
        assert_eq!(
            decode(&cursor),
            Ok(("2026-08-02T00:00:00Z".to_owned(), "app_01".to_owned()))
        );
        assert!(decode("not-base64!").is_err());
        assert!(decode(&URL_SAFE_NO_PAD.encode("missing-separator")).is_err());
    }
}
