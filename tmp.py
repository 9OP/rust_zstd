# Some helper scripts

# Welcome Telecom zstd
# RAW = [
#     0xa,
#     0x23,
#     0x20,
#     0x57,
#     0x65,
#     0x6c,
#     0x63,
#     0x6f,
#     0x6d,
#     0x65,
#     0x20,
#     0x74,
#     0x6f,
#     0x20,
#     0x54,
#     0x65,
#     0x6c,
#     0x65,
#     0x63,
#     0x6f,
#     0x6d,
#     0x20,
#     0x50,
#     0x61,
#     0x72,
#     0x69,
#     0x73,
#     0x20,
#     0x7a,
#     0x73,
#     0x74,
#     0x64,
#     0x20,
#     0x65,
#     0x78,
#     0x61,
#     0x6d,
#     0x70,
#     0x6c,
#     0x65,
#     0x20,
#     0x23,
#     0xa,
# ]


RAW = [
    0x54,
    0x68,
    0x69,
    0x73,
    0x20,
    0x69,
    0x73,
    0x20,
    0x74,
    0x68,
    0x65,
    0x20,
    0x63,
    0x6f,
    0x6e,
    0x74,
    0x65,
    0x6e,
    0x74,
    0x20,
    0x6f,
    0x66,
    0x66,
    0x69,
    0x72,
    0x73,
    0x74,
    0x20,
    0x66,
    0x69,
    0x6c,
    0x65,
    0x2e,
    0xa,
    0x73,
    0x65,
    0x63,
    0x6f,
    0x6e,
    0x64,
    0x20,
    0x66,
    0x69,
    0x6c,
    0x65,
    0x2e,
]


# ascii_string = ''.join([chr(hex_value) for hex_value in RAW])
# print(ascii_string, len(RAW))

data = """
0 	0 	5 	0
1 	6 	4 	0
2 	9 	5 	0
3 	15 	5 	0
4 	21 	5 	0
5 	3 	5 	0
6 	7 	4 	0
7 	12 	5 	0
8 	18 	5 	0
9 	23 	5 	0
10 	5 	5 	0
11 	8 	4 	0
12 	14 	5 	0
13 	20 	5 	0
14 	2 	5 	0
15 	7 	4 	16
16 	11 	5 	0
17 	17 	5 	0
18 	22 	5 	0
19 	4 	5 	0
20 	8 	4 	16
21 	13 	5 	0
22 	19 	5 	0
23 	1 	5 	0
24 	6 	4 	16
25 	10 	5 	0
26 	16 	5 	0
27 	28 	5 	0
28 	27 	5 	0
29 	26 	5 	0
30 	25 	5 	0
31 	24 	5 	0
""".strip()

out = ""
# Process each line
for line in data.split('\n'):
    # Split the line into columns
    columns = line.split()

    # Apply transformations
    state = '0x{:02x}'.format(int(columns[0]))  # Convert the first column to hex
    symbol = 's' + columns[1]  # Prefix the second column with 's'
    baseline = '0x{:02x}'.format(int(columns[3]))
    num_bits = columns[2]

    # Format and print the transformed line
    transformed_line = f"{state},{symbol},{baseline},{num_bits}"
    out += transformed_line + "\n"

print(out)
