import os


def list_folders(path):
    all_folders = []
    for root, subfolders, files in os.walk(path):
        all_folders.extend(list(map(lambda x: os.path.join(root, x), subfolders)))
    return all_folders
