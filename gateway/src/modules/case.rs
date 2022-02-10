use std::sync::Arc;
use chrono::Utc;
use futures_util::future::err;
use mongodb::bson::{DateTime, doc};
use twilight_http::Client;
use twilight_model::application::callback::CallbackData;
use twilight_model::channel::embed::Embed;
use twilight_model::guild::audit_log::{AuditLogChange, AuditLogEventType};
use twilight_model::id::Id;
use twilight_model::id::marker::{GuildMarker, UserMarker};
use twilight_util::snowflake::Snowflake;
use database::models::case::Case;
use database::mongodb::MongoDBConnection;

pub async fn run(mongodb: MongoDBConnection, discord_http: Arc<Client>, guild_id: Id<GuildMarker>, target_id: Id<UserMarker>, action_type: (AuditLogEventType, u8)) -> Result<(), ()> {

    let event_at = Utc::now().timestamp();

    let guild_config = mongodb.get_config(guild_id.clone()).await.map_err(|_| ())?;
    if !guild_config.moderation.native_support {
        return Err(())
    }

    let audit_log = discord_http
        .audit_log(guild_id)
        .action_type(action_type.0)
        .limit(1).map_err(|_| ())?
        .exec().await.map_err(|_| ())?.model().await.map_err(|_| ())?;

    let action = audit_log.entries.first().ok_or(())?;
    let action_target_id = action.target_id.ok_or(())?;
    if action_target_id.to_string() != target_id.to_string() {
        return Err(());
    };

    let change = action.changes.last().ok_or(())?;

    let duration = if action_type.1 == 7 {
        if let AuditLogChange::AfkTimeout { old, new } = change {
            Some(new.clone())
        } else {
            return Err(())
        }
    } else { None };

    let moderator = action.user_id.ok_or(())?.clone();

    let created_at = action.id.timestamp();
    let ping = created_at - event_at * 1000;

    if ping > 2000 {
        return Err(());
    }

    let count = mongodb.cases.count_documents(doc! {}, None).await.map_err(|_| ())?;

    let case = Case {
        moderator_id: moderator,
        created_at: DateTime::now(),
        guild_id,
        member_id: target_id,
        action: action_type.1,
        reason: action.reason.clone(),
        removed: false,
        duration,
        index: (count + 1) as u16
    };

    let result = mongodb.cases.insert_one(case.clone(), None).await;

    let logs_channel = guild_config.moderation.logs_channel.ok_or(())?;

    discord_http.clone().create_message(logs_channel).embeds(&[
        if let Err(error) = result {
            Embed {
                author: None,
                color: None,
                description: Some(format!("{:?}", error)),
                fields: vec![],
                footer: None,
                image: None,
                kind: "article".to_string(),
                provider: None,
                thumbnail: None,
                timestamp: None,
                title: Some("Error while creating case".to_string()),
                url: None,
                video: None
            }
        } else {
            case.to_embed(discord_http).await.map_err(|_| ())?
        }
    ]).map_err(|_| ())?.exec().await.map_err(|_| ())?;

    Ok(())

}

pub mod on_kick {
    use std::sync::Arc;
    use twilight_http::Client;
    use twilight_model::gateway::payload::incoming::MemberRemove;
    use twilight_model::guild::audit_log::AuditLogEventType;
    use database::mongodb::MongoDBConnection;

    pub async fn run(
        event: MemberRemove,
        mongodb: MongoDBConnection,
        discord_http: Arc<Client>,
    ) -> Result<(), ()> {

        crate::modules::case::run(mongodb, discord_http, event.guild_id, event.user.id, (AuditLogEventType::MemberKick, 6)).await

    }
}

pub mod on_ban {
    use database::mongodb::MongoDBConnection;
    use std::sync::Arc;
    use twilight_http::Client;
    use twilight_model::gateway::payload::incoming::BanAdd;
    use twilight_model::guild::audit_log::AuditLogEventType;

    pub async fn run(event: BanAdd, mongodb: MongoDBConnection, discord_http: Arc<Client>) -> Result<(), ()> {
        crate::modules::case::run(mongodb, discord_http, event.guild_id, event.user.id, (AuditLogEventType::MemberBanAdd, 4)).await
    }
}

pub mod on_timeout {
    use database::mongodb::MongoDBConnection;
    use std::sync::Arc;
    use twilight_http::Client;
    use twilight_model::gateway::payload::incoming::MemberUpdate;
    use twilight_model::guild::audit_log::AuditLogEventType;

    pub async fn run(_event: Box<MemberUpdate>, _mongodb: MongoDBConnection, _discord_http: Arc<Client>) -> Result<(), ()> {
        Ok(())
    }
}
