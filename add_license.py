import os

header = """// Copyright (c) 2026 Lucio Di Capua <ldcproductions@proton.me>
// Licensed under the Prosperity Public License 3.0.0
// Commercial use requires a license. See LICENSE.md for details.

"""

def add_header(directory):
    for root, _, files in os.walk(directory):
        for file in files:
            if file.endswith(".rs"):
                path = os.path.join(root, file)
                with open(path, 'r', encoding='utf-8') as f:
                    content = f.read()

                # Aggiunge l'header solo se non è già presente
                if not content.startswith("// Copyright"):
                    with open(path, 'w', encoding='utf-8') as f:
                        f.write(header + content)
                    print(f"Header aggiunto a: {file}")

add_header('src')
