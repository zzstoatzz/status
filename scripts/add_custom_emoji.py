#!/usr/bin/env python3
"""
Script to add custom emojis to the database
"""
import sqlite3
import sys
import time
from pathlib import Path

def add_custom_emoji(db_path, name, filename, alt_text=None, category="custom"):
    """Add a custom emoji to the database"""
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Check if emoji already exists
    cursor.execute("SELECT COUNT(*) FROM custom_emojis WHERE name = ?", (name,))
    if cursor.fetchone()[0] > 0:
        print(f"Emoji '{name}' already exists, skipping...")
        conn.close()
        return False
    
    # Add the emoji
    added_at = int(time.time())
    cursor.execute(
        "INSERT INTO custom_emojis (name, filename, alt_text, category, addedAt) VALUES (?, ?, ?, ?, ?)",
        (name, filename, alt_text, category, added_at)
    )
    
    conn.commit()
    conn.close()
    print(f"Added emoji '{name}' -> {filename}")
    return True

def main():
    # Default database path
    db_path = "status.db"
    
    # Example custom emojis to add
    emojis = [
        ("partyparrot", "partyparrot.gif", "Party Parrot", "custom"),
        ("shipit", "shipit.png", "Ship It Squirrel", "custom"),
        ("blobheart", "blobheart.png", "Blob Heart", "custom"),
        ("rustacean", "rustacean.png", "Rust Crab", "custom"),
        ("dumpsterfire", "dumpsterfire.gif", "Dumpster Fire", "custom"),
    ]
    
    print(f"Adding custom emojis to {db_path}...")
    
    for name, filename, alt_text, category in emojis:
        add_custom_emoji(db_path, name, filename, alt_text, category)
    
    print("Done!")

if __name__ == "__main__":
    main()