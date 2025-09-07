#!/usr/bin/env python3
"""
Actually test the OAuth flow end-to-end
"""
import requests
import json
import sys
from urllib.parse import urlparse, parse_qs

def test_real_oauth(handle, app_password):
    """Test the actual OAuth flow"""
    
    print("=" * 60)
    print("TESTING REAL OAUTH FLOW")
    print("=" * 60)
    
    # Step 1: Create a session to simulate being logged into Bluesky
    print("\n1. Creating Bluesky session...")
    session = requests.Session()
    
    login_response = session.post(
        "https://bsky.social/xrpc/com.atproto.server.createSession",
        json={
            "identifier": handle,
            "password": app_password
        }
    )
    
    if login_response.status_code != 200:
        print(f"Failed to login: {login_response.text}")
        return
    
    login_data = login_response.json()
    did = login_data['did']
    access_token = login_data['accessJwt']
    print(f"✓ Logged in as {did}")
    
    # Step 2: Start OAuth authorization
    print("\n2. Starting OAuth flow...")
    
    client_id = "https://zzstoatzz-status-pr-32.fly.dev/oauth-client-metadata.json"
    redirect_uri = "https://zzstoatzz-status-pr-32.fly.dev/oauth/callback"
    
    # Test different scope combinations
    test_scopes = [
        ("Current (getFollows no fragment)", "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"),
        ("Both with fragment", "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview"),
        ("Both without fragment", "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"),
        ("Just getProfile (working in prod)", "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview"),
    ]
    
    for scope_name, scope in test_scopes:
        print(f"\nTesting: {scope_name}")
        print(f"Scope: {scope}")
        
        auth_params = {
            "response_type": "code",
            "client_id": client_id,
            "redirect_uri": redirect_uri,
            "scope": scope,
            "state": "test123"
        }
        
        # Make the authorization request
        auth_response = session.get(
            "https://bsky.social/oauth/authorize",
            params=auth_params,
            allow_redirects=False,
            headers={"Authorization": f"Bearer {access_token}"}
        )
        
        print(f"  Status: {auth_response.status_code}")
        
        if auth_response.status_code == 400:
            # Try to extract error from HTML
            import re
            error_match = re.search(r'<title>(.*?)</title>', auth_response.text)
            if error_match:
                print(f"  Error: {error_match.group(1)}")
            continue
        elif auth_response.status_code != 302:
            print(f"  Unexpected status")
            continue
            
        # If we got here, it worked
        print(f"  ✓ SUCCESS! This scope configuration works!")
        
        # Continue with the working scope
        scope = scope  # Use the last tested scope
    
    print(f"Authorization response status: {auth_response.status_code}")
    
    if auth_response.status_code == 302:
        # Check if we got redirected with a code
        location = auth_response.headers.get('Location')
        if location:
            parsed = urlparse(location)
            params = parse_qs(parsed.query)
            
            if 'code' in params:
                code = params['code'][0]
                print(f"✓ Got authorization code: {code[:20]}...")
                
                # Step 3: Exchange code for token
                print("\n3. Exchanging code for token...")
                
                token_response = requests.post(
                    "https://bsky.social/oauth/token",
                    json={
                        "grant_type": "authorization_code",
                        "code": code,
                        "redirect_uri": redirect_uri,
                        "client_id": client_id
                    }
                )
                
                print(f"Token exchange status: {token_response.status_code}")
                if token_response.status_code == 200:
                    token_data = token_response.json()
                    oauth_token = token_data.get('access_token')
                    print(f"✓ Got OAuth token")
                    
                    # Decode the token to see what scopes it has
                    if oauth_token:
                        import base64
                        parts = oauth_token.split('.')
                        if len(parts) >= 2:
                            payload = parts[1]
                            # Add padding if needed
                            padding = 4 - len(payload) % 4
                            if padding != 4:
                                payload += '=' * padding
                            try:
                                decoded = base64.urlsafe_b64decode(payload)
                                token_payload = json.loads(decoded)
                                print("\nToken payload:")
                                print(json.dumps(token_payload, indent=2))
                                
                                if 'scope' in token_payload:
                                    print(f"\n✓ Scopes in token: {token_payload['scope']}")
                                else:
                                    print("\n✗ No scope field in token!")
                            except:
                                print("Could not decode token payload")
                    
                    # Step 4: Test the token
                    print("\n4. Testing OAuth token with APIs...")
                    
                    # Test getProfile
                    profile_resp = requests.get(
                        "https://bsky.social/xrpc/app.bsky.actor.getProfile",
                        params={"actor": did},
                        headers={"Authorization": f"Bearer {oauth_token}"}
                    )
                    print(f"getProfile: {profile_resp.status_code}")
                    if profile_resp.status_code != 200:
                        print(f"  Error: {profile_resp.text[:200]}")
                    
                    # Test getFollows
                    follows_resp = requests.get(
                        "https://bsky.social/xrpc/app.bsky.graph.getFollows",
                        params={"actor": did, "limit": 1},
                        headers={"Authorization": f"Bearer {oauth_token}"}
                    )
                    print(f"getFollows: {follows_resp.status_code}")
                    if follows_resp.status_code != 200:
                        print(f"  Error: {follows_resp.text[:200]}")
                    
                else:
                    print(f"✗ Token exchange failed: {token_response.text}")
            
            elif 'error' in params:
                print(f"✗ Got error: {params.get('error')} - {params.get('error_description')}")
            else:
                print(f"✗ Unexpected redirect: {location}")
        else:
            print("✗ No redirect location")
    else:
        print(f"Response headers: {dict(auth_response.headers)}")
        print(f"Response body: {auth_response.text[:500]}")
    
    print("\n" + "=" * 60)
    print("CONCLUSION:")
    print("This test shows what scopes the OAuth token actually gets")
    print("vs what we're requesting in the metadata.")
    print("=" * 60)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python test_oauth_real.py <handle> <app_password>")
        print("Example: python test_oauth_real.py alice.bsky.social myapppassword")
        sys.exit(1)
    
    handle = sys.argv[1]
    app_password = sys.argv[2]
    
    test_real_oauth(handle, app_password)