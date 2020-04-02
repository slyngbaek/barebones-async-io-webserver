use std::collections::HashMap;

type MatchType = Option<HashMap<String, String>>;

pub fn match_path(path: &str, pat: &str) -> MatchType {
    let split_path = path.split("?").collect::<Vec<&str>>();
    let split_pat = pat.split("?").collect::<Vec<&str>>();

    let mut vars = HashMap::new();
    vars = match_part_path(split_path[0], split_pat[0], vars)?;
    match_part_params(split_path.get(1), split_pat.get(1), vars)
}

fn match_part_path(path: &str, pat: &str, mut vars: HashMap<String, String>) -> MatchType {
    let path = path.split("/").collect::<Vec<&str>>();
    let pat = pat.split("/").collect::<Vec<&str>>();
    if path.len() != pat.len() {
        return None;
    }
    for (path, pat) in path.iter().zip(pat.iter()) {
        if pat.starts_with("{") && pat.ends_with("}") {
            vars.insert(pat[1..pat.len() - 1].to_string(), path.to_string());
        } else if path != pat {
            return None;
        }
    }
    Some(vars)
}

fn match_part_params(
    path_params: Option<&&str>,
    pat_params: Option<&&str>,
    mut vars: HashMap<String, String>,
) -> MatchType {
    let path_params = parse_query_params(path_params);
    let pat_params = parse_query_params(pat_params);

    for (pat_key, pat_val) in pat_params.iter() {
        if pat_val.starts_with("{") && pat_val.ends_with("}") && path_params.get(pat_key).is_some()
        {
            vars.insert(
                pat_val[1..pat_val.len() - 1].to_string(),
                path_params[pat_key].to_string(),
            );
        } else if path_params.get(pat_key) != Some(pat_val) {
            return None;
        }
    }

    Some(vars)
}

fn parse_query_params(params: Option<&&str>) -> HashMap<String, String> {
    let mut query_params = HashMap::new();
    if let Some(params) = params {
        for param in params.split("&") {
            let param = param.split("=").collect::<Vec<&str>>();
            query_params.insert(param[0].into(), param[1].into());
        }
    }
    query_params
}
