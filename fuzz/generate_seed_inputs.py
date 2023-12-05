import os

def get_files_as_csv(directory_path):    
    files = [directory_path + "/" + f for f in os.listdir(directory_path) if os.path.isfile(os.path.join(directory_path, f))]
    csv_string = ','.join(files)
    return csv_string


# Replace "/path/to/directory" with the actual path of the directory
directory_path = "./tests/corpus"
result = get_files_as_csv(directory_path)


print(result)



