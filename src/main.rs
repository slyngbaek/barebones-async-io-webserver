mod chat;
mod server;

use chat::{Chat, Contacts, Message};
use http::header::CONTENT_TYPE;
use http::Method;
use server::Server;
use std::collections::{HashMap, HashSet};

fn main() {
    let contacts: Contacts = serde_json::from_str(include_str!("contacts.json")).unwrap();
    let mut chats: HashMap<i64, Chat> = HashMap::new();
    let mut users: HashMap<i64, HashSet<i64>> = HashMap::new();

    Server::new(|req, res| {
        println!("req {:?} {:?}", req.method(), req.uri().to_string());

        match (req.method(), req.uri().path_and_query().unwrap().as_str()) {
            (&Method::GET, path) => {
                if let Some(vars) = match_path(path, "/chats?userId={user_id}") {
                    let user_id = vars["user_id"].parse::<i64>()?;
                    let chats: Vec<&Chat> = users
                        .get(&user_id)
                        .ok_or("No chats for user")?
                        .iter()
                        .filter_map(|chat_id| chats.get(chat_id))
                        .collect();

                    return Ok(res
                        .status(200)
                        .header(CONTENT_TYPE, "application/json")
                        .body(serde_json::to_string(&chats)?)?);
                } else if let Some(vars) = match_path(path, "/chats/{chat_id}/messages") {
                    let chat_id = vars["chat_id"].parse::<i64>()?;
                    let messages: &Vec<Message> =
                        &chats.get(&chat_id).ok_or("Chat does not exist")?.messages;

                    return Ok(res
                        .status(200)
                        .header(CONTENT_TYPE, "application/json")
                        .body(serde_json::to_string(messages)?)?);
                }
            }
            (&Method::POST, path) => {
                if let Some(_) = match_path(path, "/chats") {
                    let (_parts, body) = req.into_parts();
                    let chat: Chat = serde_json::from_str(&body)?;

                    users
                        .entry(chat.participant_ids[0])
                        .or_insert(HashSet::new())
                        .insert(chat.id);
                    users
                        .entry(chat.participant_ids[1])
                        .or_insert(HashSet::new())
                        .insert(chat.id);
                    chats.insert(
                        chat.id,
                        Chat::new(chat.id, [chat.participant_ids[0], chat.participant_ids[1]]),
                    );

                    return Ok(res.status(200).body(String::new())?);
                } else if let Some(vars) = match_path(path, "/chats/{chat_id}/messages") {
                    let (_parts, body) = req.into_parts();
                    let message: Message = serde_json::from_str(&body)?;
                    let _is_valid_dest_contact = contacts
                        .get(&message.source_user_id)
                        .map(|contacts| contacts.contains(&message.destination_user_id))
                        .ok_or("Can't send message to contact not in address list")?;

                    chats
                        .get_mut(&vars["chat_id"].parse::<i64>()?)
                        .ok_or("Missing Chat")?
                        .add_message(message);

                    return Ok(res.status(200).body(String::new())?);
                }
            }
            _ => {}
        }

        Ok(res.status(404).body(String::new())?)
    })
    .listen("127.0.0.1:6000");
}

fn match_path(path: &str, pat: &str) -> Option<HashMap<String, String>> {
    let split_path = path.split("?").collect::<Vec<&str>>();
    let split_pat = pat.split("?").collect::<Vec<&str>>();

    let mut vars = HashMap::new();

    let is_same_len = split_path[0].split("/").count() == split_pat[0].split("/").count();
    let is_path_match = is_same_len
        && split_path[0]
            .split("/")
            .zip(split_pat[0].split("/"))
            .all(|(a, b)| {
                if b.starts_with("{") && b.ends_with("}") {
                    let var_name = &b[1..b.len() - 1];
                    vars.insert(var_name.to_string(), a.to_string());
                    true
                } else {
                    a == b
                }
            });

    let is_param_match = (split_path.len() == 1 && split_pat.len() == 1)
        || (split_pat.len() > 1 && split_path.len() > 1 && {
            let path_params = split_path[1]
                .split("&")
                .fold(HashMap::new(), |mut m, param| {
                    let param = param.split("=").collect::<Vec<&str>>();
                    m.insert(param[0], param[1]);
                    m
                });
            let pat_params = split_pat[1]
                .split("&")
                .fold(HashMap::new(), |mut m, param| {
                    let param = param.split("=").collect::<Vec<&str>>();
                    m.insert(param[0], param[1]);
                    m
                });

            pat_params.iter().all(|(k, v)| {
                if let Some(val) = path_params.get(k) {
                    if v.starts_with("{") && v.ends_with("}") {
                        let var_name = &v[1..v.len() - 1];
                        vars.insert(var_name.to_string(), val.to_string());
                        true
                    } else {
                        path_params.get(k) == Some(v)
                    }
                } else {
                    false
                }
            })
        });

    if is_path_match && is_param_match {
        Some(vars)
    } else {
        None
    }
}
