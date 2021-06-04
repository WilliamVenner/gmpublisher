use std::sync::Arc;

use crate::{GMOD_APP_ID, WorkshopItem, webview::Addon};
use parking_lot::Mutex;
use steamworks::{QueryResults, SteamError};

#[tauri::command]
pub fn browse_subscribed_addons(page: u32) -> Option<(u32, Vec<Addon>)> {
	let results = Arc::new(Mutex::new(None));

	let results_ref = results.clone();
	let client = steam!().client();
	client
		.ugc()
		.query_user(
			client.steam_id.account_id(),
			steamworks::UserList::Subscribed,
			steamworks::UGCType::ItemsReadyToUse,
			steamworks::UserListOrder::SubscriptionDateDesc,
			steamworks::AppIDs::ConsumerAppId(GMOD_APP_ID),
			page,
		)
		.ok()?
		.require_tag("addon")
		.fetch(move |result: Result<QueryResults<'_>, SteamError>| {
			if let Ok(data) = result {
				*results_ref.lock() = Some(Some((
					data.total_results(),
					data.iter()
						.enumerate()
						.map(|(i, x)| {
							let mut item: WorkshopItem = x.unwrap().into();
							item.preview_url = data.preview_url(i as u32);
							item.subscriptions = data.statistic(i as u32, steamworks::UGCStatisticType::Subscriptions).unwrap_or(0);
							item.into()
						})
						.collect::<Vec<Addon>>(),
				)));
			} else {
				*results_ref.lock() = Some(None);
			}
		});

	mutex_wait!(results, {
		steam!().run_callbacks();
	});

	Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}
