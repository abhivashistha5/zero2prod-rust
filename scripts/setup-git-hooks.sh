#!/bin/bash

# Find root directory of the project
ROOT_DIR=$(git rev-parse --show-toplevel)

# Change working directory to ROOT_DIR
cd "$ROOT_DIR" || exit

echo "Changed working directory to: $ROOT_DIR"


# Directory where hooks are stored
HOOKS_DIR="$ROOT_DIR/scripts/hooks"

# Function to install hooks
install_hooks() {
    # Check if hooks directory exists
    if [ ! -d "$HOOKS_DIR" ]; then
        echo "Hooks directory not found."
        exit 1
    fi

    # Iterate over hooks files
    for hook_file in "$HOOKS_DIR"/*; do
        hook_name=$(basename "$hook_file")

        # Check if hook file is executable
        if [ -x "$hook_file" ]; then
            # Check if hook file already exists in git hooks directory
            if [ -f ".git/hooks/$hook_name" ]; then
                echo "Hook '$hook_name' already exists. Skipping..."
            else
                # Create symbolic link to hook file
                ln -s "$hook_file" ".git/hooks/$hook_name"
                echo "Hook '$hook_name' installed."
            fi
        else
            echo "Skipping non-executable file '$hook_name'. Make sure it's executable."
        fi
    done

    echo "Hooks installation completed."
}

# Execute installation
install_hooks

