use ezgif_server::discord::interactions::{
    autocomplete_choices, ephemeral_message, plain_message,
};

#[test]
fn plain_message_can_be_public_or_private() {
    let public = plain_message("https://example.com/cat.gif", false);
    let private = plain_message("Missing category.", true);

    assert_eq!(public["type"], 4);
    assert_eq!(public["data"]["content"], "https://example.com/cat.gif");
    assert!(public["data"].get("flags").is_none());
    assert_eq!(private["data"]["flags"], 64);
}

#[test]
fn errors_are_ephemeral() {
    let error = ephemeral_message("That category has no saved links yet.");

    assert_eq!(error["type"], 4);
    assert_eq!(error["data"]["flags"], 64);
}

#[test]
fn autocomplete_choices_truncate_to_discord_limit() {
    let values = (0..30)
        .map(|index| (format!("Choice {index}"), format!("value-{index}")))
        .collect();

    let response = autocomplete_choices(values);
    let choices = response["data"]["choices"].as_array().unwrap();

    assert_eq!(response["type"], 8);
    assert_eq!(choices.len(), 25);
    assert_eq!(choices[0]["name"], "Choice 0");
    assert_eq!(choices[24]["value"], "value-24");
}
