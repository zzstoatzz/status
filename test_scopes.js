#!/usr/bin/env node

// Test OAuth scope validation with Bluesky
const testCases = [
    {
        name: "Both with fragment",
        metadata_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview",
        request_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview"
    },
    {
        name: "getFollows without fragment in both",
        metadata_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app",
        request_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    },
    {
        name: "getFollows without fragment in request only",
        metadata_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app#bsky_appview",
        request_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app#bsky_appview rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    },
    {
        name: "Both without fragment",
        metadata_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app",
        request_scope: "atproto repo:io.zzstoatzz.status.record rpc:app.bsky.actor.getProfile?aud=did:web:api.bsky.app rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app"
    }
];

console.log("OAuth Scope Test Results");
console.log("=" .repeat(60));

for (const test of testCases) {
    console.log(`\nTest: ${test.name}`);
    console.log(`Metadata scope: ${test.metadata_scope}`);
    console.log(`Request scope:  ${test.request_scope}`);
    
    // Check if scopes match
    const matches = test.metadata_scope === test.request_scope;
    console.log(`Scopes match: ${matches ? "✓" : "✗"}`);
    
    // Check what error message would be
    if (test.request_scope.includes("getFollows?aud=did:web:api.bsky.app#bsky_appview")) {
        console.log("Expected error: Missing scope without fragment");
    } else if (test.request_scope.includes("getFollows?aud=did:web:api.bsky.app")) {
        console.log("Expected: Should work if metadata declares it");
    }
    
    console.log("-".repeat(40));
}

console.log("\nCONCLUSION:");
console.log("The error message says it needs: rpc:app.bsky.graph.getFollows?aud=did:web:api.bsky.app");
console.log("This is WITHOUT the #bsky_appview fragment");
console.log("\nWe should try:");
console.log("1. Metadata AND request both WITHOUT fragment for getFollows");
console.log("2. Or check if there's a different issue entirely");