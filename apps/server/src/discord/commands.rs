use serde_json::{Value, json};

pub fn command_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "mb",
            "description": "Send a random image from one of your buckets",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "bucket",
                    "description": "Your bucket, or comma-separated buckets",
                    "type": 3,
                    "required": true,
                    "autocomplete": true
                },
                {
                    "name": "private",
                    "description": "Only show the response to you",
                    "type": 5,
                    "required": false
                },
                {
                    "name": "target",
                    "description": "User to ping with the GIF",
                    "type": 6,
                    "required": false
                }
            ]
        }),
        json!({
            "name": "bucket",
            "description": "Manage your image buckets",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "create",
                    "description": "Create a bucket",
                    "type": 1,
                    "options": [
                        {"name": "name", "description": "Bucket name", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "add",
                    "description": "Add an image URL to a bucket",
                    "type": 1,
                    "options": [
                        {"name": "bucket", "description": "Your bucket", "type": 3, "required": true, "autocomplete": true},
                        {"name": "url", "description": "Image or GIF URL", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "list",
                    "description": "List your buckets",
                    "type": 1
                }
            ]
        }),
        json!({
            "name": "manage",
            "description": "Open the web dashboard to manage your buckets",
            "integration_types": [1],
            "contexts": [0, 1, 2]
        }),
        json!({
            "name": "Add to Bucket",
            "type": 3,
            "integration_types": [1],
            "contexts": [0, 1, 2]
        }),
        json!({
            "name": "Reply with GIF",
            "type": 3,
            "integration_types": [1],
            "contexts": [0, 1, 2]
        }),
    ]
}
