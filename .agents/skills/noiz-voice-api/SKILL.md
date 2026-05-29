---
name: noiz-voice-api
description: Integrate Noiz Voice AI APIs — voice cloning, text-to-speech with emotion control, voice design, and management. Use when building voice apps, generating speech, or cloning voices.
license: MIT
---

# Noiz Voice API

Voice cloning, text-to-speech with emotion, and voice design — one API for agents.

## Skill Files

| File | URL |
| --- | --- |
| SKILL.md (this file) | `https://developers.noiz.ai/SKILL.md` |
| OpenAPI Spec | `https://developers.noiz.ai/openapi.yaml` |

Base URL: `https://noiz.ai/v1`
Auth: `Authorization: YOUR_API_KEY` header on every request.

## Endpoints Overview

### 1. Voice Management

| Method | Path | Description |
| --- | --- | --- |
| GET | `/voices` | List voices (custom or built-in) |
| POST | `/voices` | Clone a voice from file or URL |
| GET | `/voices/{voice_id}` | Get voice details |
| DELETE | `/voices/{voice_id}` | Soft delete a voice |

### 2. Text to Speech

| Method | Path | Description |
| --- | --- | --- |
| POST | `/text-to-speech` | Convert text to speech (returns audio binary) |
| POST | `/emotion-enhance` | Auto-annotate text with emotion tags |
| POST | `/guest/text-to-speech` | Guest TTS (no auth, rate-limited, voice_id optional) |

### 3. Text to Sound

| Method | Path | Description |
| --- | --- | --- |
| POST | `/text-to-sound` | Generate a sound effect from a text prompt |
| GET | `/text-to-sound-history` | List past generated sound effects |
| DELETE | `/text-to-sound-history/{gen_product_id}` | Delete a sound effect record |

### 4. Voice Design

| Method | Path | Description |
| --- | --- | --- |
| POST | `/voice-design` | Design a voice from text description or image |

## Response Format

All JSON responses: `{ code: 0, message: "success", data: {...} }`
TTS returns raw audio binary.

## Error Codes

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 400 | Bad request |
| 401 | Missing or invalid API key |
| 402 | Insufficient credits |
| 404 | Voice not found |
| 429 | Rate limit |
| 500 | Server error |

## Workflow

1. Clone or design a voice → get `voice_id`
2. (Optional) Enhance text with `/emotion-enhance`
3. Synthesize with `/text-to-speech` using `voice_id` + text
4. Manage voices via list / get / delete
