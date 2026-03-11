#!/usr/bin/env python3
import os
import re

# Paths to process
ROOT_DIR = '/Users/axient/repository/AxiomMe'

# Extensions to process
TARGET_EXTENSIONS = {'.rs', '.toml', '.md', '.yaml', '.sh', '.gitignore'}

# Ignore directories
IGNORE_DIRS = {'.git', 'target', '.gemini'}

# Replacements (Order matters to avoid substring overlap issues, usually longest first, but here it's case sensitive)
REPLACEMENTS = [
    ('AxiomMe', 'AxiomNexus'),
    ('axiomme', 'axiomnexus'),
    ('AXIOMME', 'AXIOMNEXUS'),
]

def process_file(filepath):
    try:
        with open(filepath, 'r', encoding='utf-8') as f:
            content = f.read()
    except Exception as e:
        print(f"Skipping {filepath} due to read error: {e}")
        return

    new_content = content
    for old_str, new_str in REPLACEMENTS:
        new_content = new_content.replace(old_str, new_str)
        
    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated: {filepath}")

def walk_and_replace():
    for root, dirs, files in os.walk(ROOT_DIR):
        # Modify dirs in place to skip ignored directories
        dirs[:] = [d for d in dirs if d not in IGNORE_DIRS]
        
        for file in files:
            # Check extension, or if it's a file without extension like 'process-compose.yaml'
            ext = os.path.splitext(file)[1]
            if ext in TARGET_EXTENSIONS or file in {'.gitignore', 'rust-toolchain.toml'}:
                filepath = os.path.join(root, file)
                process_file(filepath)

if __name__ == '__main__':
    walk_and_replace()
    print("Done bulk replacing text.")
