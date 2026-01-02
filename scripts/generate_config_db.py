#!/usr/bin/env python3
"""
One-time script to generate config_db from mackup repository.

Usage:
    python3 scripts/generate_config_db.py [mackup_repo_path] [output_dir]

Example:
    python3 scripts/generate_config_db.py ~/mackup ./config_db
"""

import sys
import configparser
from pathlib import Path
from typing import Dict, Optional


def parse_mackup_cfg(cfg_path: Path) -> Optional[Dict[str, any]]:
    """Parse a mackup .cfg file and extract relevant sections."""
    parser = configparser.ConfigParser()
    
    try:
        parser.read(cfg_path)
    except Exception as e:
        print(f"Warning: Failed to parse {cfg_path}: {e}")
        return None
    
    result = {
        'name': None,
        'configuration_files': [],
        'xdg_configuration_files': []
    }
    
    # Extract application name
    if 'application' in parser:
        result['name'] = parser['application'].get('name', '')
    
    # Extract configuration files
    if 'configuration_files' in parser:
        for key in parser['configuration_files']:
            value = parser['configuration_files'][key]
            if value and value.strip():
                result['configuration_files'].append(value.strip())
    
    # Extract XDG configuration files
    if 'xdg_configuration_files' in parser:
        for key in parser['xdg_configuration_files']:
            value = parser['xdg_configuration_files'][key]
            if value and value.strip():
                result['xdg_configuration_files'].append(value.strip())
    
    return result


def create_flat_structure(stub_name: str, data: Dict[str, any], output_dir: Path):
    """Create flat file structure from parsed mackup data."""
    
    # Create directories
    apps_dir = output_dir / 'applications'
    config_files_dir = output_dir / 'configuration_files'
    xdg_files_dir = output_dir / 'xdg_configuration_files'
    
    apps_dir.mkdir(parents=True, exist_ok=True)
    config_files_dir.mkdir(parents=True, exist_ok=True)
    xdg_files_dir.mkdir(parents=True, exist_ok=True)
    
    # Write application name
    if data['name']:
        app_file = apps_dir / f"{stub_name}.conf"
        with open(app_file, 'w') as f:
            f.write(f"name = {data['name']}\n")
    
    # Write configuration files
    config_file = config_files_dir / f"{stub_name}.conf"
    if data['configuration_files']:
        with open(config_file, 'w') as f:
            for cfg in data['configuration_files']:
                f.write(f"{cfg}\n")
    else:
        # Create empty file
        config_file.touch()
    
    # Write XDG configuration files
    xdg_file = xdg_files_dir / f"{stub_name}.conf"
    if data['xdg_configuration_files']:
        with open(xdg_file, 'w') as f:
            for cfg in data['xdg_configuration_files']:
                f.write(f"{cfg}\n")
    else:
        # Create empty file
        xdg_file.touch()


def process_mackup_repo(mackup_path: Path, output_dir: Path):
    """Process all .cfg files in mackup repository."""
    
    # Find the applications directory in mackup
    apps_source = mackup_path / 'mackup' / 'applications'
    
    if not apps_source.exists():
        # Try alternative paths
        apps_source = mackup_path / 'src' / 'mackup' / 'applications'
    
    if not apps_source.exists():
        print(f"Error: Could not find applications directory in {mackup_path}")
        print("Expected: mackup/applications or src/mackup/applications")
        sys.exit(1)
    
    print(f"Processing mackup applications from: {apps_source}")
    
    # Process each .cfg file
    cfg_files = list(apps_source.glob('*.cfg'))
    print(f"Found {len(cfg_files)} .cfg files")
    
    processed = 0
    skipped = 0
    
    for cfg_file in cfg_files:
        stub_name = cfg_file.stem  # filename without extension
        
        # Parse the cfg file
        data = parse_mackup_cfg(cfg_file)
        
        if data is None:
            skipped += 1
            continue
        
        # Skip if no useful data
        if not data['name'] and not data['configuration_files'] and not data['xdg_configuration_files']:
            skipped += 1
            continue
        
        # Create flat structure
        create_flat_structure(stub_name, data, output_dir)
        processed += 1
        
        if processed % 50 == 0:
            print(f"Processed {processed} applications...")
    
    print(f"\n✓ Successfully processed {processed} applications")
    print(f"⚠ Skipped {skipped} applications")
    print(f"📁 Output directory: {output_dir.absolute()}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 scripts/generate_config_db.py [mackup_repo_path] [output_dir]")
        print("\nExample:")
        print("  python3 scripts/generate_config_db.py ~/mackup ./config_db")
        print("\nIf mackup is not cloned, run:")
        print("  git clone https://github.com/lra/mackup.git ~/mackup")
        sys.exit(1)
    
    mackup_path = Path(sys.argv[1]).expanduser().resolve()
    output_dir = Path(sys.argv[2] if len(sys.argv) > 2 else './config_db').resolve()
    
    if not mackup_path.exists():
        print(f"Error: Mackup repository not found at {mackup_path}")
        print("\nClone it with:")
        print("  git clone https://github.com/lra/mackup.git ~/mackup")
        sys.exit(1)
    
    print("=" * 60)
    print("Mackup to config_db converter")
    print("=" * 60)
    print(f"Source: {mackup_path}")
    print(f"Output: {output_dir}")
    print()
    
    # Create output directory
    output_dir.mkdir(parents=True, exist_ok=True)
    
    # Process repository
    process_mackup_repo(mackup_path, output_dir)
    
    print("\n✓ Done! Config database generated successfully.")
    print("\nYou can now use the generated stubs with:")
    print("  dotfiles add <stub-name>")


if __name__ == '__main__':
    main()
