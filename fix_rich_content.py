import re

with open('src/routes/rich_content.rs', 'r') as f:
    content = f.read()

# For `Ok(Json(json!({...})))` we want to replace with `Ok(ApiResponse::success(json!({...})))` or `Ok(ApiResponse::success_with_message(...))` if it has a message.
# Since it's a bit complex, let's do simple regex replacements for common patterns.
# Pattern 1: success with data
# Ok(Json(json!({ "success": true, "data": ... }))) -> Ok(ApiResponse::success(...))

# Let's just use ApiResponse::success(serde_json::json!({...})) as a fallback, or better:
# Ok((StatusCode::CREATED, Json(json!({"success": true, "message": msg, "data": data})))) -> Ok((StatusCode::CREATED, ApiResponse::success_with_message(msg, data)))

# Since it's rust, we can use rust-analyzer or just write a slightly smarter script.

import os

def process_file(filepath):
    with open(filepath, 'r') as f:
        text = f.read()
        
    text = re.sub(r'use serde_json::json;', r'use serde_json::json;\nuse crate::dto::response_dto::ApiResponse;', text)
    
    # 1. Ok(Json(json!({ "success": true, "data": <expr> }))) -> Ok(ApiResponse::success(<expr>))
    text = re.sub(
        r'Ok\(Json\(json!\(\{\s*"success": true,\s*"data": (.*?)\s*\}\)\)\)',
        r'Ok(ApiResponse::success(\1))',
        text,
        flags=re.DOTALL
    )
    
    # 2. Ok((StatusCode::CREATED, Json(json!({ "success": true, "message": <msg>, "data": <expr> })))) -> Ok((StatusCode::CREATED, ApiResponse::success_with_message(<msg>, <expr>)))
    text = re.sub(
        r'Ok\(\(StatusCode::CREATED, Json\(json!\(\{\s*"success": true,\s*"message": (.*?),\s*"data": (.*?)\s*\}\)\)\)\)',
        r'Ok((StatusCode::CREATED, ApiResponse::success_with_message(\1, \2)))',
        text,
        flags=re.DOTALL
    )

    # 3. Ok(Json(json!({ "success": true, "message": <msg> }))) -> Ok(ApiResponse::success_with_message(<msg>, json!({})))
    text = re.sub(
        r'Ok\(Json\(json!\(\{\s*"success": true,\s*"message": (.*?)\s*\}\)\)\)',
        r'Ok(ApiResponse::success_with_message(\1, json!({})))',
        text,
        flags=re.DOTALL
    )

    with open(filepath, 'w') as f:
        f.write(text)

process_file('src/routes/rich_content.rs')

