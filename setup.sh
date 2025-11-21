#!/bin/bash
set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up Project Deep Archive environment...${NC}"

# 1. Directory Structure
echo "Checking directories..."
mkdir -p models
mkdir -p data
mkdir -p .output
echo -e "${GREEN}✔ Directories created/verified (models, data, .output)${NC}"

# 2. System Dependencies
echo "Checking system dependencies..."
MISSING_DEPS=0

if ! command -v ffmpeg &> /dev/null; then
    echo -e "${RED}✘ ffmpeg is not installed.${NC}"
    MISSING_DEPS=1
else
    echo -e "${GREEN}✔ ffmpeg is installed.${NC}"
fi

if ! command -v xorriso &> /dev/null; then
    echo -e "${RED}✘ xorriso is not installed.${NC}"
    MISSING_DEPS=1
else
    echo -e "${GREEN}✔ xorriso is installed.${NC}"
fi

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "${YELLOW}Please install missing dependencies:${NC}"
    echo "  Debian/Ubuntu: sudo apt install ffmpeg xorriso"
    echo "  macOS: brew install ffmpeg xorriso"
    echo "  Fedora: sudo dnf install ffmpeg xorriso"
    echo "  Arch: sudo pacman -S ffmpeg xorriso"
    # We don't exit here, we allow the script to continue to download models,
    # but the user is warned.
fi

# 3. Model Downloads
echo "Checking AI models..."

download_model() {
    local url=$1
    local dest=$2

    if [ -f "$dest" ]; then
        echo -e "${GREEN}✔ $dest already exists.${NC}"
    else
        echo -e "${YELLOW}Downloading $dest...${NC}"
        if command -v curl &> /dev/null; then
            curl -L "$url" -o "$dest"
        elif command -v wget &> /dev/null; then
            wget "$url" -O "$dest"
        else
            echo -e "${RED}✘ neither curl nor wget found. Cannot download models.${NC}"
            exit 1
        fi

        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✔ Downloaded $dest${NC}"
        else
            echo -e "${RED}✘ Failed to download $dest${NC}"
            exit 1
        fi
    fi
}

download_model "https://huggingface.co/Falconsai/nsfw_image_detection/resolve/main/model.onnx" "models/nsfw.onnx"
download_model "https://huggingface.co/SmilingWolf/wd-v1-4-convnext-tagger-v2/resolve/main/model.onnx" "models/tagger.onnx"

# 4. Permissions (Ensure this script is executable)
# This is a bit meta since the script is already running, but good for future runs if copied.
chmod +x "$0"

echo -e "${GREEN}Setup complete!${NC}"
echo "You can now run the project using the command found in README.md"
