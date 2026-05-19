import re
import hashlib
import os

from src.list_files import list_files
from src.parse_args import parse_cmd_args

args = parse_cmd_args()
print(f'args: {str(args)}')


def calc_file_hash(file_path):
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        # Read and update hash string value in blocks of 4K
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
        return sha256_hash.hexdigest()


def extract_existing_hash(input_filename):
    result_match = re.search(r"([^.]*)(\.?.*)", input_filename)
    if result_match is None:
        return False
    no_ext_file_path = result_match.group(1)
    result_match = re.search(r".*?([a-fA-F0-9]{6,})$", no_ext_file_path)
    if result_match is not None:
        return result_match.group(1)
    return None


files = list_files(args.path)
if os.path.isfile(args.path):
    files.append(args.path)

if args.hash:
    print("** HASH **")
    for file in files:
        if extract_existing_hash(file) is not None:
            print(f'Skipping existing file: {file}')
            continue
        path_match = re.search(r"([^.]*)(\.?.*)", file)
        no_extension_file_path = path_match.group(1)
        extension = path_match.group(2)
        file_hash = calc_file_hash(file).upper()
        print(f'hash: {file_hash} - file: {file}')

        os.rename(file, no_extension_file_path + "_SHA256_" + file_hash[0:args.hash_length] + extension)


elif args.verify:
    print("** VERIFY **")
    for file in files:
        extracted_file_hash = extract_existing_hash(file)
        if extracted_file_hash is None:
            continue
        extracted_file_hash = extracted_file_hash.lower()
        file_hash = calc_file_hash(file)[0:len(extracted_file_hash)]
        if extracted_file_hash.__eq__(file_hash):
            print(f'all_good - file: {file}')
        else:
            print(f'HASH MISMATCH!!! from name: {extracted_file_hash} - actual: {file_hash} (file: "{file}")')
