use twilight_model::application::callback::CallbackData;
use twilight_model::channel::embed::Embed;

fn text_to_embed(title: String, description: String) -> Embed {
    Embed {
        author: None,
        color: None,
        description: Some(description),
        fields: vec![],
        footer: None,
        image: None,
        kind: "".to_string(),
        provider: None,
        thumbnail: None,
        timestamp: None,
        title: Some(title),
        url: None,
        video: None
    }
}

pub fn text_to_response_embed(title: String, description: String) -> CallbackData {
    CallbackData {
        allowed_mentions: None,
        components: None,
        content: None,
        embeds: Some(vec![text_to_embed(title, description)]),
        flags: None,
        tts: None
    }
}