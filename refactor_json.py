import os

def process_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    if 'Ok(Json(json!({' not in content:
        return
        
    print(f"Processing {filepath}")
    
    idx = 0
    while True:
        idx = content.find('Ok(Json(json!({', idx)
        if idx == -1:
            break
            
        start = idx
        brace_count = 0
        in_string = False
        escape = False
        
        inner_start = content.find('{', start)
        
        i = start
        while i < len(content):
            char = content[i]
            if char == '"' and not escape:
                in_string = not in_string
            elif char == '\\' and in_string:
                escape = not escape
            else:
                escape = False
                
            if not in_string:
                if char == '(':
                    brace_count += 1
                elif char == ')':
                    brace_count -= 1
                    if brace_count == 0:
                        old_expr = content[start:i+1]
                        
                        # Fix: Don't add an extra ')' at the end, just wrap the inner part correctly.
                        # Wait, the inner part is `json!({...})`. 
                        # We are replacing `Ok(Json(X))` with `Ok(ApiResponse::success(X))`.
                        # So we can just replace `Ok(Json(` with `Ok(crate::dto::response_dto::ApiResponse::success(`
                        # No need to add closing parentheses! The existing closing parentheses for `Ok` and `Json` will map perfectly to `Ok` and `ApiResponse::success`.
                        # Wait! `Json(...)` has one `(`. `ApiResponse::success(...)` has one `(`.
                        # So `Ok(Json(` -> `Ok(crate::dto::response_dto::ApiResponse::success(`.
                        # It's an exact structural replacement!
                        
                        # So we just do a simple text replace!
                        break
            i += 1
            
    # Actually, if we just replace `Ok(Json(json!({` with `Ok(crate::dto::response_dto::ApiResponse::success(serde_json::json!({`, the parentheses match exactly!
    # Because `Ok(` (1) `Json(` (2) `json!(` (3)
    # -> `Ok(` (1) `ApiResponse::success(` (2) `json!(` (3)
    # The closing `)))` will close all three correctly!
    
    # Wait, what if it was `serde_json::json!`?
    content = content.replace("Ok(Json(json!(", "Ok(crate::dto::response_dto::ApiResponse::success(serde_json::json!(")
    
    with open(filepath, 'w') as f:
        f.write(content)

for root, _, files in os.walk('src/routes'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))

