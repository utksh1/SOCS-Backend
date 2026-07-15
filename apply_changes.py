import sys

def apply_changes():
    with open('changes.txt', 'r') as f:
        lines = f.readlines()
        
    current_file = None
    target = []
    replacement = []
    state = 0 # 0=none, 1=target, 2=replacement
    
    def apply_current():
        if not current_file or not target:
            return
            
        t_str = '\n'.join(target)
        r_str = '\n'.join(replacement)
        
        try:
            with open(current_file, 'r') as cf:
                content = cf.read()
                
            if t_str in content:
                content = content.replace(t_str, r_str)
                with open(current_file, 'w') as cf:
                    cf.write(content)
                print(f"Applied change to {current_file}")
            else:
                print(f"Target not found in {current_file}")
        except Exception as e:
            print(f"Error {e}")
            
    for line in lines:
        line = line.rstrip('\n')
        if line.startswith('File: '):
            if current_file and target:
                apply_current()
            current_file = line[6:].strip()
            target = []
            replacement = []
            state = 0
        elif line == '--- TARGET ---':
            if target and state == 2:
                apply_current()
                target = []
                replacement = []
            state = 1
        elif line == '--- REPLACEMENT ---':
            state = 2
        elif line == '========================================':
            pass
        else:
            if state == 1:
                target.append(line)
            elif state == 2:
                replacement.append(line)
                
    if current_file and target:
        apply_current()

apply_changes()
