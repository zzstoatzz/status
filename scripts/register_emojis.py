#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""
Register all downloaded emoji images in the database
"""

import sqlite3
import time
from pathlib import Path


def main():
    # Setup paths
    script_dir = Path(__file__).parent
    project_root = script_dir.parent
    emojis_dir = project_root / "static" / "emojis"
    db_path = project_root / "statusphere.sqlite3"
    
    if not db_path.exists():
        print(f"Error: Database not found at {db_path}")
        return
    
    # Get all image files
    image_files = []
    for ext in ['*.png', '*.gif', '*.jpg', '*.jpeg', '*.webp']:
        image_files.extend(emojis_dir.glob(ext))
    
    print(f"Found {len(image_files)} image files")
    
    # Connect to database
    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()
    
    # Check what already exists
    cursor.execute("SELECT name FROM custom_emojis")
    existing = {row[0] for row in cursor.fetchall()}
    print(f"Already registered: {len(existing)} emojis")
    
    # Register new emojis
    added = 0
    skipped = 0
    timestamp = int(time.time())
    
    for image_path in image_files:
        filename = image_path.name
        # Create a short name from filename
        name = filename.rsplit('.', 1)[0]
        # Truncate super long names
        if len(name) > 50:
            name = name[:47] + "..."
        
        if name in existing:
            skipped += 1
            continue
        
        # Determine mime type
        ext = filename.rsplit('.', 1)[-1].lower()
        mime_map = {
            'png': 'image/png',
            'gif': 'image/gif',
            'jpg': 'image/jpeg',
            'jpeg': 'image/jpeg',
            'webp': 'image/webp'
        }
        mime_type = mime_map.get(ext, 'image/png')
        
        # Create alt text from name
        alt_text = name.replace('-', ' ').replace('_', ' ')
        
        cursor.execute(
            "INSERT INTO custom_emojis (name, filename, alt_text, category, addedAt) VALUES (?, ?, ?, ?, ?)",
            (name, filename, alt_text, 'bufo', timestamp)
        )
        added += 1
    
    conn.commit()
    conn.close()
    
    print(f"✓ Added {added} new emojis")
    if skipped:
        print(f"  Skipped {skipped} existing emojis")
    
    print(f"\nTotal emojis in database now: {len(existing) + added}")


if __name__ == "__main__":
    main()