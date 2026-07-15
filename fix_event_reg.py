import re

with open('src/routes/event_registrations.rs', 'r') as f:
    content = f.read()

content = content.replace(
"""let registration = event_registration_repository::register(
        &state.db,
        event_id,
        &payload.name,
        &payload.email,
        None,
    ).await""",
"""let email = crate::utils::sanitize::normalize_email(&payload.email);
    let registration = event_registration_repository::register(
        &state.db,
        event_id,
        &payload.name,
        &email,
        None,
    ).await""")

with open('src/routes/event_registrations.rs', 'w') as f:
    f.write(content)
