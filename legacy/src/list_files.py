import os


def list_files(path):
    all_files = []
    for root, subfolders, files in os.walk(path):
        all_files.extend(list(map(lambda x: os.path.join(root, x), files)))
    return all_files
