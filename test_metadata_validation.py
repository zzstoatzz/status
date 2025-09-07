#!/usr/bin/env python3
"""
Check if Bluesky can fetch and validate our metadata
"""
import requests
import json

def test_metadata_validation():
    """Test if the metadata is valid and accessible"""
    
    print("=" * 60)
    print("METADATA VALIDATION TEST")
    print("=" * 60)
    
    metadata_url = "https://zzstoatzz-status-pr-32.fly.dev/oauth-client-metadata.json"
    
    # 1. Fetch the metadata
    print(f"\n1. Fetching metadata from: {metadata_url}")
    response = requests.get(metadata_url)
    
    if response.status_code != 200:
        print(f"✗ Failed to fetch metadata: {response.status_code}")
        return
    
    metadata = response.json()
    print("✓ Metadata fetched successfully")
    
    # 2. Check what Bluesky would see
    print("\n2. Metadata content:")
    print(json.dumps(metadata, indent=2))
    
    # 3. Check if the metadata is being cached
    print("\n3. Testing if metadata is cached...")
    headers = {
        "User-Agent": "Bluesky OAuth Client",
        "Accept": "application/json"
    }
    
    # Make multiple requests to see if it's consistent
    for i in range(3):
        r = requests.get(metadata_url, headers=headers)
        if r.status_code == 200:
            data = r.json()
            print(f"  Request {i+1}: scope = {data['scope'][:50]}...")
        else:
            print(f"  Request {i+1}: Failed with {r.status_code}")
    
    # 4. Check if there's a mismatch between what we declare and what we request
    print("\n4. Checking scope matching:")
    declared_scope = metadata['scope']
    
    # Split scopes for comparison
    declared_scopes = set(declared_scope.split())
    print(f"  Declared scopes ({len(declared_scopes)}):")
    for s in declared_scopes:
        print(f"    - {s}")
    
    # What we're trying to request
    requested_scope = "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    requested_scopes = set(requested_scope.split())
    print(f"\n  Requested scopes ({len(requested_scopes)}):")
    for s in requested_scopes:
        print(f"    - {s}")
    
    # Compare
    print("\n5. Comparison:")
    if declared_scopes == requested_scopes:
        print("✓ Scopes match exactly")
    else:
        print("✗ Scopes don't match!")
        missing = requested_scopes - declared_scopes
        if missing:
            print(f"  Missing from metadata: {missing}")
        extra = declared_scopes - requested_scopes
        if extra:
            print(f"  Extra in metadata: {extra}")
    
    # 6. Test if the issue is URL encoding
    print("\n6. URL Encoding check:")
    import urllib.parse
    
    encoded_scope = urllib.parse.quote(declared_scope)
    print(f"  URL encoded scope: {encoded_scope[:100]}...")
    
    # Check if fragment is causing issues
    if "#" in declared_scope:
        print("  ⚠️  Scope contains # fragment which might need special handling")
    if "?" in declared_scope:
        print("  ⚠️  Scope contains ? query params which might need special handling")
    
    print("\n" + "=" * 60)
    print("HYPOTHESIS:")
    print("The 400 error means Bluesky can't validate our OAuth request.")
    print("Possible reasons:")
    print("1. Metadata isn't accessible to Bluesky")
    print("2. Scope format is incorrect")
    print("3. Client ID doesn't match redirect URI domain")
    print("4. The OAuth client needs to be registered differently")
    print("=" * 60)

if __name__ == "__main__":
    test_metadata_validation()