# SOCS API Documentation

Complete API reference for the SOCS backend.

---

## Base URL

**Development**: `http://localhost:5001/api`  
**Production**: `https://api.socs.network/api`

---

## Authentication

All protected endpoints require a JWT token in the Authorization header:

```
Authorization: Bearer <your-jwt-token>
```

### Obtaining a Token

**Login**:
```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

**Response**:
```json
{
  "success": true,
  "message": "Login successful",
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "user": {
      "id": "uuid",
      "name": "John Doe",
      "email": "user@example.com",
      "role": "MEMBER"
    }
  }
}
```

---

## Response Format

All API responses follow this standard format:

**Success**:
```json
{
  "success": true,
  "message": "Optional message",
  "data": { ... }
}
```

**Error**:
```json
{
  "success": false,
  "message": "Error message",
  "errors": {
    "field": ["Error detail"]
  }
}
```

**Paginated**:
```json
{
  "success": true,
  "data": {
    "items": [...],
    "pagination": {
      "page": 1,
      "limit": 10,
      "total": 100,
      "totalPages": 10
    }
  }
}
```

---

## Endpoints

### Authentication

#### Register User
```http
POST /api/auth/register
```

**Body**:
```json
{
  "name": "John Doe",
  "email": "john@example.com",
  "password": "SecurePass123"
}
```

**Response**: User object + token

---

#### Login
```http
POST /api/auth/login
```

**Body**:
```json
{
  "email": "john@example.com",
  "password": "SecurePass123"
}
```

---

#### Get Current User
```http
GET /api/auth/me
Authorization: Bearer <token>
```

---

### Projects

#### List Projects
```http
GET /api/projects?page=1&limit=10&search=security&featured=true&tag=Web%20Security
```

**Query Parameters**:
- `page` (number): Page number (default: 1)
- `limit` (number): Items per page (default: 10, max: 100)
- `search` (string): Search in title/description
- `featured` (boolean): Filter by featured status
- `tag` (string): Filter by tag
- `createdById` (string): Filter by creator

---

#### Get Project
```http
GET /api/projects/:id
```

**Parameters**:
- `:id` - Project UUID or slug

---

#### Create Project
```http
POST /api/projects
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**:
```json
{
  "slug": "project-nightshade",
  "title": "Project Nightshade",
  "description": "Advanced vulnerability scanner",
  "techStack": ["Python", "Go", "Docker"],
  "tags": ["Web Security", "Automation"],
  "githubLink": "https://github.com/socs/nightshade",
  "featured": false
}
```

---

#### Update Project
```http
PUT /api/projects/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**: Same as create (partial updates allowed)

---

#### Delete Project
```http
DELETE /api/projects/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

### Events

#### List Events
```http
GET /api/events?page=1&limit=10&status=upcoming&type=ctf&fromDate=2026-01-01&toDate=2026-12-31
```

**Query Parameters**:
- `page`, `limit`: Pagination
- `search`: Search in title/description
- `status`: `upcoming` or `past`
- `type`: `workshop`, `ctf`, `talk`, `hackathon`
- `fromDate`, `toDate`: Date range filter

---

#### Get Event
```http
GET /api/events/:id
```

---

#### Create Event
```http
POST /api/events
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**:
```json
{
  "slug": "spring-ctf-2026",
  "title": "Spring Security CTF",
  "description": "Annual capture the flag competition",
  "date": "2026-05-22T10:00:00Z",
  "type": "ctf",
  "status": "upcoming",
  "location": "Virtual"
}
```

---

#### Update Event
```http
PUT /api/events/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

#### Delete Event
```http
DELETE /api/events/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

### Team Members

#### List Team Members
```http
GET /api/team?page=1&limit=10&search=alex&tier=core
```

**Query Parameters**:
- `page`, `limit`: Pagination
- `search`: Search in name/role
- `tier`: `core`, `lead`, or `member`

---

#### Get Team Member
```http
GET /api/team/:id
```

---

#### Create Team Member
```http
POST /api/team
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**:
```json
{
  "slug": "alex-vance",
  "name": "Alex Vance",
  "role": "President",
  "skills": ["Python", "Web Security", "Leadership"],
  "tier": "core",
  "github": "https://github.com/alexvance",
  "linkedin": "https://linkedin.com/in/alexvance",
  "image": "https://example.com/avatar.jpg"
}
```

---

#### Update Team Member
```http
PUT /api/team/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

#### Delete Team Member
```http
DELETE /api/team/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

### Resources

#### List Resources
```http
GET /api/resources?page=1&limit=10&category=tool&search=burp
```

**Query Parameters**:
- `page`, `limit`: Pagination
- `search`: Search in title/description
- `category`: `tool`, `roadmap`, `writeup`, `blog`

---

#### Get Resource
```http
GET /api/resources/:id
```

---

#### Create Resource
```http
POST /api/resources
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**:
```json
{
  "title": "OWASP Top 10 Guide",
  "description": "Comprehensive guide to OWASP Top 10",
  "url": "https://owasp.org/top10",
  "category": "roadmap",
  "tags": ["Web", "Beginner"]
}
```

---

#### Update Resource
```http
PUT /api/resources/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

#### Delete Resource
```http
DELETE /api/resources/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

### Visuals

#### List Visuals
```http
GET /api/visuals?page=1&limit=10&category=TEAM
```

**Query Parameters**:
- `page`, `limit`: Pagination
- `search`: Search in title
- `category`: `TEAM`, `INFRA`, `EVENT`

---

#### Get Visual
```http
GET /api/visuals/:id
```

---

#### Create Visual
```http
POST /api/visuals
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

**Body**:
```json
{
  "title": "LAB_SNAPSHOT_01",
  "category": "TEAM",
  "src": "https://images.unsplash.com/photo-..."
}
```

---

#### Update Visual
```http
PUT /api/visuals/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

#### Delete Visual
```http
DELETE /api/visuals/:id
Authorization: Bearer <token>
Roles: ADMIN, MANAGEMENT
```

---

### Contact

#### Submit Contact Form
```http
POST /api/contacts
```

**Body**:
```json
{
  "name": "Jane Doe",
  "email": "jane@example.com",
  "message": "I'd like to collaborate on a project..."
}
```

---

## Rate Limiting

**General API**: 300 requests per 15 minutes  
**Auth Endpoints**: 10 requests per 15 minutes

**Rate Limit Headers**:
- `X-RateLimit-Limit`: Total requests allowed
- `X-RateLimit-Remaining`: Remaining requests
- `X-RateLimit-Reset`: Time when limit resets

---

## Error Codes

| Status Code | Meaning |
|-------------|---------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request (validation error) |
| 401 | Unauthorized (no token or invalid token) |
| 403 | Forbidden (insufficient permissions) |
| 404 | Not Found |
| 409 | Conflict (duplicate email, slug, etc.) |
| 422 | Unprocessable Entity |
| 429 | Too Many Requests (rate limit exceeded) |
| 500 | Internal Server Error |

---

## Swagger Documentation

Interactive API documentation available at:

**Development**: `http://localhost:5001/api-docs`  
**Production**: `https://api.socs.network/api-docs`

---

## Examples

### Complete Flow: Create a Project

1. **Login**:
```bash
curl -X POST http://localhost:5001/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@socs.network",
    "password": "SocsAdmin@2026"
  }'
```

2. **Extract Token** from response

3. **Create Project**:
```bash
curl -X POST http://localhost:5001/api/projects \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-token>" \
  -d '{
    "slug": "new-project",
    "title": "New Project",
    "description": "A cool new project",
    "techStack": ["Python"],
    "tags": ["Web Security"],
    "featured": false
  }'
```

---

## SDKs and Libraries

**JavaScript/TypeScript**:
```typescript
import { API_BASE_URL } from './config';

async function fetchProjects() {
  const response = await fetch(`${API_BASE_URL}/projects?limit=10`);
  const json = await response.json();
  return json.data;
}
```

**Python** (example using requests):
```python
import requests

BASE_URL = "http://localhost:5001/api"

def login(email, password):
    response = requests.post(f"{BASE_URL}/auth/login", json={
        "email": email,
        "password": password
    })
    return response.json()["data"]["token"]

def get_projects(token):
    response = requests.get(f"{BASE_URL}/projects", headers={
        "Authorization": f"Bearer {token}"
    })
    return response.json()["data"]["items"]
```

---

## Webhooks (Future)

Webhooks are not currently implemented but planned for future releases to notify external services of:
- New applications
- Project updates
- Event registrations

---

## Changelog

**v1.0.0** (Current)
- Initial API release
- All CRUD endpoints for core entities
- JWT authentication
- Role-based authorization
- Audit logging

**Planned for v1.1.0**:
- Global search endpoint
- Bulk operations
- Webhook support
- GraphQL API (optional)
