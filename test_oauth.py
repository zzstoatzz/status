#!/usr/bin/env python3
import requests
import json

# Test different scope combinations
test_cases = [
    {
        "name": "Both with fragment",
        "scope": "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview"
    },
    {
        "name": "getFollows without fragment",
        "scope": "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    },
    {
        "name": "Both without fragment",
        "scope": "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    },
    {
        "name": "getProfile without fragment",
        "scope": "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview"
    }
]

print("Testing OAuth scope combinations...")
print("=" * 60)

for test in test_cases:
    print(f"\nTest: {test['name']}")
    print(f"Scope: {test['scope']}")
    
    # Create mock client metadata
    metadata = {
        "client_id": "https://test.example.com/oauth-client-metadata.json",
        "client_name": "Test Status App",
        "client_uri": "https://test.example.com",
        "redirect_uris": ["https://test.example.com/oauth/callback"],
        "scope": test['scope'],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
        "dpop_bound_access_tokens": True
    }
    
    print(f"Metadata valid: {json.dumps(metadata, indent=2)}")
    print("-" * 40)