# This does not work if a comment is put like this:
# add %1, r0, r1 ; a comment
def replace_asm(file):
    with open(file, 'r') as f:
        lines = f.readlines()
    i = 0
    str_to_replace = '%1'
    already_replaced = False
    new_files = []
    for line in lines:
        if str_to_replace in line and ';' not in line: 
            new_strs = find_new_strs(lines[i-1],str_to_replace)

            if not already_replaced:
                already_replaced = True
                for _ in range(len(new_strs)):
                    new_files.append(lines.copy())

            for j,new_file in enumerate(new_files):
                new_file[i] = line.replace(str_to_replace, new_strs[j])
        i += 1

    for file in new_files:
        print("FILE")
        for line in file:
            print(line)


def find_new_strs(line,str_to_replace):
    values = line.split(";")
    for value in values:
        if str_to_replace in value:
            value_stripped = value.replace(str_to_replace + "=","").strip("\n").strip(" ")
            return value_stripped.split(",")              



replace_asm('./programs/add_test.zasm')
