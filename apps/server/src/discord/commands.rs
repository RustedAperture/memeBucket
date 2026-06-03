use serde_json::{json, Value};

pub fn command_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "random",
            "description": "Send a random media link from one of your categories",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "category",
                    "description": "Your category",
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
            "description": "Manage your media pools",
            "integration_types": [1],
            "contexts": [0, 1, 2],
            "options": [
                {
                    "name": "create",
                    "description": "Create a category",
                    "type": 1,
                    "options": [
                        {"name": "name", "description": "Category name", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "add",
                    "description": "Add a URL to a category",
                    "type": 1,
                    "options": [
                        {"name": "category", "description": "Your category", "type": 3, "required": true, "autocomplete": true},
                        {"name": "url", "description": "Image or GIF URL", "type": 3, "required": true}
                    ]
                },
                {
                    "name": "list",
                    "description": "List your categories",
                    "type": 1
                }
            ]
        }),
    ]
}
