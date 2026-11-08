use std::collections::{HashMap, HashSet};
use std::time::Duration;

use steam_vent::{Connection, ConnectionTrait};
use steam_vent_proto_steam::enums_clientserver::EMsg;
use steam_vent_proto_steam::steammessages_clientserver::CMsgClientLicenseList;
use steam_vent_proto_steam::steammessages_clientserver_appinfo::{
    CMsgClientPICSProductInfoRequest, CMsgClientPICSProductInfoResponse,
    cmsg_client_picsproduct_info_request,
};
use tokio::time::timeout;

use crate::kv;

const CHUNK: usize = 500;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub type Licenses = Vec<(u32, u64)>;

pub fn licenses_from(list: &CMsgClientLicenseList) -> Licenses {
    list.licenses
        .iter()
        .map(|l| (l.package_id(), l.access_token()))
        .collect()
}

pub fn buffered_licenses(connection: &Connection) -> Option<Licenses> {
    connection
        .take_unprocessed()
        .into_iter()
        .filter(|raw| raw.kind == EMsg::k_EMsgClientLicenseList)
        .filter_map(|raw| raw.into_message::<CMsgClientLicenseList>().ok())
        .last()
        .map(|list| licenses_from(&list))
}

pub async fn owned_among(
    connection: &Connection,
    licenses: &Licenses,
    wanted: &[u32],
) -> Option<HashSet<u32>> {
    let mut owned = HashSet::new();

    for chunk in licenses.chunks(CHUNK) {
        let request = CMsgClientPICSProductInfoRequest {
            packages: chunk
                .iter()
                .map(
                    |&(id, token)| cmsg_client_picsproduct_info_request::PackageInfo {
                        packageid: Some(id),
                        access_token: Some(token),
                        ..Default::default()
                    },
                )
                .collect(),
            meta_data_only: Some(false),
            single_response: Some(true),
            ..Default::default()
        };

        let response: CMsgClientPICSProductInfoResponse =
            timeout(REQUEST_TIMEOUT, connection.job(request))
                .await
                .ok()?
                .ok()?;

        for package in &response.packages {
            let Some(buffer) = package.buffer.as_deref().and_then(|b| b.get(4..)) else {
                continue;
            };
            let Some(root) = kv::parse(buffer) else {
                continue;
            };
            let Some(apps) = root.first_section().and_then(|p| p.get("appids")) else {
                continue;
            };

            owned.extend(
                apps.children()
                    .iter()
                    .filter_map(|(_, v)| v.as_u32())
                    .filter(|id| wanted.contains(id)),
            );
        }

        if wanted.iter().all(|id| owned.contains(id)) {
            break;
        }
    }

    Some(owned)
}

pub async fn names(connection: &Connection, apps: &[u32]) -> HashMap<u32, String> {
    let request = CMsgClientPICSProductInfoRequest {
        apps: apps
            .iter()
            .map(|&id| cmsg_client_picsproduct_info_request::AppInfo {
                appid: Some(id),
                ..Default::default()
            })
            .collect(),
        meta_data_only: Some(false),
        single_response: Some(true),
        ..Default::default()
    };

    let Ok(Ok(response)) = timeout(
        REQUEST_TIMEOUT,
        connection.job::<_, CMsgClientPICSProductInfoResponse>(request),
    )
    .await
    else {
        return HashMap::new();
    };

    response
        .apps
        .iter()
        .filter_map(|app| {
            let text = String::from_utf8_lossy(app.buffer.as_deref()?);
            Some((app.appid(), common_name(&text)?))
        })
        .collect()
}

fn common_name(vdf: &str) -> Option<String> {
    let common = vdf.find("\"common\"")?;
    let rest = &vdf[common..];
    let key = rest.find("\"name\"")?;
    let rest = &rest[key + "\"name\"".len()..];
    let start = rest.find('"')? + 1;
    let end = start + rest[start..].find('"')?;
    let name = rest[start..end].trim();

    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::common_name;

    #[test]
    fn reads_common_name() {
        let vdf = "\"appinfo\"\n{\n\t\"appid\"\t\t\"4162040\"\n\t\"common\"\n\t{\n\t\t\"name\"\t\t\"Zenless Zone Zero\"\n\t\t\"type\"\t\t\"Game\"\n\t}\n}";
        assert_eq!(common_name(vdf).as_deref(), Some("Zenless Zone Zero"));
        assert_eq!(common_name("\"appinfo\" { }"), None);
    }
}
