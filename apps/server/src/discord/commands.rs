use serde_json::{Value, json};

pub fn command_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "ez",
            "description": "Send a random image from one of your pools",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "pool",
                    "description": "Your pool, or comma-separated pools",
                    "type": 3,
                    "required": true,
                    "autocomplete": true
                },
                {
                    "name": "private",
                    "description": "Only show the response to you",
                    "type": 5,
                    "required": false
                }
            ]
        }),
        json!({
            "name": "pool",
            "description": "Manage your image pools",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "create",
                    "description": "Create a pool",
                    "type": 1,
                    "options": [
                        {"name": "name", "description": "Pool name", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "add",
                    "description": "Add an image URL to a pool",
                    "type": 1,
                    "options": [
                        {"name": "pool", "description": "Your pool", "type": 3, "required": true, "autocomplete": true},
                        {"name": "url", "description": "Image or GIF URL", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "list",
                    "description": "List your pools",
                    "type": 1
                }
            ]
        }),
        json!({
            "name": "manage",
            "description": "Open the web dashboard to manage your pools",
            "integration_types": [1],
            "contexts": [0, 1, 2]
        }),
        json!({
            "name": "Add to Pool",
            "type": 3,
            "integration_types": [1],
            "contexts": [0, 1, 2]
        }),
    ]
}
