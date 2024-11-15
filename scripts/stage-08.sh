file_dir="../bin/file_directory"
cd "$(dirname "$0")"
mkdir -p $file_dir
curl -v -X "POST" --data "1234567890" -H "Content-Type: application/octet-stream" http://localhost:4221/files/file_123
