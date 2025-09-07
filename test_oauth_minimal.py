#!/usr/bin/env python3
"""
Minimal test to check if OAuth metadata and scope declarations work
"""
import requests
import json
from urllib.parse import urlencode

def test_oauth_metadata():
    """Test if the OAuth metadata is correctly configured"""
    
    print("=" * 60)
    print("OAuth Metadata and Scope Test")
    print("=" * 60)
    
    # Test production
    print("\n1. Production site metadata:")
    prod_url = "https://status.zzstoatzz.io/oauth-client-metadata.json"
    prod_response = requests.get(prod_url)
    if prod_response.status_code == 200:
        prod_metadata = prod_response.json()
        print(f"✓ Got metadata")
        print(f"  Scope: {prod_metadata['scope']}")
        
        # Check which scopes are present
        scope = prod_metadata['scope']
        if "rpc:app.bsky.actor.getProfile" in scope:
            has_fragment = "#bsky_appview" in scope.split("rpc:app.bsky.actor.getProfile")[1].split()[0]
            print(f"  - getProfile: present (fragment: {has_fragment})")
        
        if "rpc:app.bsky.graph.getFollows" in scope:
            has_fragment = "#bsky_appview" in scope.split("rpc:app.bsky.graph.getFollows")[1] if "rpc:app.bsky.graph.getFollows" in scope else False
            print(f"  - getFollows: present (fragment: {has_fragment})")
    else:
        print(f"✗ Failed to get metadata: {prod_response.status_code}")
    
    # Test preview
    print("\n2. Preview site metadata:")
    preview_url = "https://zzstoatzz-status-pr-32.fly.dev/oauth-client-metadata.json"
    preview_response = requests.get(preview_url)
    if preview_response.status_code == 200:
        preview_metadata = preview_response.json()
        print(f"✓ Got metadata")
        print(f"  Scope: {preview_metadata['scope']}")
        
        # Check which scopes are present
        scope = preview_metadata['scope']
        if "rpc:app.bsky.actor.getProfile" in scope:
            profile_part = scope.split("rpc:app.bsky.actor.getProfile")[1].split()[0] if len(scope.split("rpc:app.bsky.actor.getProfile")) > 1 else ""
            has_fragment = "#bsky_appview" in profile_part
            print(f"  - getProfile: present (fragment: {has_fragment})")
        
        if "rpc:app.bsky.graph.getFollows" in scope:
            follows_part = scope.split("rpc:app.bsky.graph.getFollows")[1] if len(scope.split("rpc:app.bsky.graph.getFollows")) > 1 else ""
            has_fragment = "#bsky_appview" in follows_part
            print(f"  - getFollows: present (fragment: {has_fragment})")
    else:
        print(f"✗ Failed to get metadata: {preview_response.status_code}")
    
    # Compare
    print("\n3. Comparison:")
    if prod_response.status_code == 200 and preview_response.status_code == 200:
        if prod_metadata['scope'] == preview_metadata['scope']:
            print("✓ Scopes are identical")
        else:
            print("✗ Scopes differ:")
            print(f"  Production:  {prod_metadata['scope']}")
            print(f"  Preview:     {preview_metadata['scope']}")
    
    # Test what Bluesky expects
    print("\n4. What Bluesky error messages tell us:")
    print("  - getProfile needs: rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview")
    print("  - getFollows needs: rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app")
    print("  Note: getProfile has #bsky_appview fragment, getFollows does NOT")
    
    print("\n5. OAuth authorization URL test:")
    client_id = "https://zzstoatzz-status-pr-32.fly.dev/oauth-client-metadata.json"
    redirect_uri = "https://zzstoatzz-status-pr-32.fly.dev/oauth/callback"
    
    # Build the authorization URL with the scopes
    scope = "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    
    params = {
        "response_type": "code",
        "client_id": client_id,
        "redirect_uri": redirect_uri,
        "scope": scope,
        "state": "test"
    }
    
    auth_url = f"https://bsky.social/oauth/authorize?{urlencode(params)}"
    print(f"  Authorization URL (first 150 chars):")
    print(f"  {auth_url[:150]}...")
    
    print("\n" + "=" * 60)
    print("IMPORTANT:")
    print("The OAuth flow might be caching tokens server-side.")
    print("Even with correct scopes, old tokens might persist.")
    print("=" * 60)

if __name__ == "__main__":
    test_oauth_metadata()