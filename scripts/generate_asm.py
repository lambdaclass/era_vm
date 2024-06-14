import sys
import re

def replace_asm(file):
    """
    Creates all the possible files with all the combinations of the replacements
    """
    with open(file, 'r') as f:
        lines = f.readlines()
    replacements = get_replacements(lines)
    files_to_replace = [lines.copy()]
    
    for str_to_replace in replacements:
        new_files_to_replace = []
        for _file in files_to_replace:
            new_files = replace_once(_file,str_to_replace)
            new_files_to_replace.extend(new_files)
        if len(new_files_to_replace) != 0:
            files_to_replace = new_files_to_replace

    for j,_file in enumerate(files_to_replace):
        with open(file[:-5] + "_replaced_" + str(j) + ".zasm", 'w') as f:
            for line in _file:
                f.write(line)

def replace_once(lines,str_to_replace):
    """
    Given one file and the string to replace, it generates all the possible files with the replacements
    """
    i = 0
    already_replaced = False
    new_files = []
    for line in lines:
        if str_to_replace in line and ';' not in line:  # If it is a comment, ignore it
            new_strs = find_new_strs(lines[i-1],str_to_replace)

            if not already_replaced:
                already_replaced = True
                for _ in range(len(new_strs)):
                    new_files.append(lines.copy())

            for j,new_file in enumerate(new_files):
                new_file[i] = line.replace(str_to_replace, new_strs[j])
        i += 1
    return new_files

def find_new_strs(line,str_to_replace):
    """
    Finds the new strings to replace the %x in the line
    """
    values = line.split(";")
    for value in values:
        if str_to_replace in value:
            value_stripped = value.replace(str_to_replace + "=","").strip("\n").strip(" ")
            return value_stripped.split(",")              

def get_replacements(file):
    """
    Finds all the posible %x replacements in the file
    """
    replacements = []
    for line in file:
        replacements.extend(re.findall("%[0-9][0-9]*",line))
    return list(set(replacements))

def main():
    if len(sys.argv) != 2:
        print("Usage: python generate_asm.py <file>")
        sys.exit(1)
    replace_asm(sys.argv[1])

if __name__ == "__main__":
    main()
