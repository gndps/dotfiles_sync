#!/usr/bin/env python3
"""
Script to generate default_db.json from mackup repository.
This creates a single JSON file to be embedded in the binary.

Usage:
    python3 scripts/generate_default_db.py
"""

import sys
import json
import shutil
import subprocess
from pathlib import Path

MACKUP_REPO = "https://github.com/lra/mackup.git"
TEMP_DIR = "/tmp/mackup_for_dotfiles"
OUTPUT_FILE = "./src/default_db.json"

def clone_mackup():
    """Clone mackup repository to temp directory."""
    if Path(TEMP_DIR).exists():
        print(f"Removing existing {TEMP_DIR}...")
        shutil.rmtree(TEMP_DIR)
    
    print("Cloning mackup repository...")
    result = subprocess.run(
        ["git", "clone", "--depth=1", MACKUP_REPO, TEMP_DIR],
        capture_output=True
    )
    
    if result.returncode != 0:
        print(f"Error cloning: {result.stderr.decode()}")
        sys.exit(1)
    
    print("✓ Cloned successfully")

def find_apps_dir():
    """Find the applications directory in mackup."""
    possible_paths = [
        Path(TEMP_DIR) / "mackup" / "applications",
        Path(TEMP_DIR) / "src" / "mackup" / "applications",
    ]
    
    for path in possible_paths:
        if path.exists():
            return path
    
    print("Error: Could not find applications directory")
    sys.exit(1)

def parse_cfg_file(cfg_path):
    """Parse a mackup .cfg file."""
    data = {
        'name': None,
        'config_files': [],
    }
    
    try:
        with open(cfg_path, 'r') as f:
            content = f.read()
    except Exception:
        return None
    
    current_section = None
    
    for line in content.splitlines():
        line = line.strip()
        
        # Skip empty lines and comments
        if not line or line.startswith('#') or line.startswith(';'):
            continue
        
        # Section header
        if line.startswith('[') and line.endswith(']'):
            current_section = line[1:-1]
            continue
        
        # Parse key = value in application section
        if current_section == 'application' and '=' in line:
            key, value = line.split('=', 1)
            if key.strip() == 'name':
                data['name'] = value.strip()
        
        # Configuration files (just values, no keys)
        elif current_section == 'configuration_files':
            if line and not line.startswith('['):
                data['config_files'].append(line)
        
        # XDG files (only if no traditional files)
        elif current_section == 'xdg_configuration_files' and not data['config_files']:
            if line and not line.startswith('['):
                data['config_files'].append(line)
    
    return data


def process_mackup():
    """Process all .cfg files from mackup and generate JSON."""
    apps_dir = find_apps_dir()
    
    cfg_files = list(apps_dir.glob("*.cfg"))
    print(f"Found {len(cfg_files)} application configs")
    
    database = {}
    processed = 0
    skipped = 0
    
    for cfg_file in cfg_files:
        stub_name = cfg_file.stem
        data = parse_cfg_file(cfg_file)
        
        if not data or (not data['name'] and not data['config_files']):
            skipped += 1
            continue
        
        database[stub_name] = {
            'name': data['name'] or stub_name.replace('-', ' ').title(),
            'config_files': data['config_files']
        }
        
        processed += 1
        
        if processed % 50 == 0:
            print(f"Processed {processed}...")
    
    # Write JSON file
    output_path = Path(OUTPUT_FILE)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    with open(output_path, 'w') as f:
        json.dump(database, f, indent=2, sort_keys=True)
    
    print(f"\n✓ Processed {processed} applications")
    print(f"⚠ Skipped {skipped} applications")
    print(f"📁 Output: {output_path.absolute()}")
    print(f"📊 File size: {output_path.stat().st_size / 1024:.1f} KB")

def cleanup():
    """Remove temporary directory."""
    if Path(TEMP_DIR).exists():
        shutil.rmtree(TEMP_DIR)

def main():
    print("=" * 60)
    print("Mackup to default_db converter")
    print("=" * 60)
    print()
    
    try:
        clone_mackup()
        process_mackup()
        cleanup()
        
        print("\n✓ Done! Default database JSON generated successfully.")
        print(f"\nThe file {OUTPUT_FILE} contains 500+ application configs.")
        print("This file is embedded in the binary at compile time.")
        print("Commit this file to git before building releases.")
        
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        cleanup()
        sys.exit(1)
    except Exception as e:
        print(f"\nError: {e}")
        cleanup()
        sys.exit(1)

if __name__ == '__main__':
    main()
