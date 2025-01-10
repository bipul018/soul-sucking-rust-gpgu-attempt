
set -x
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
# glslc "$1" -o "$1.spv"
glslc "$SCRIPT_DIR/$1" -o "$SCRIPT_DIR/$1.spv"
