import json

with open('/Users/Utkarsh/.gemini/antigravity-ide/brain/bcc28373-a82a-4381-b224-351e44f543cf/.system_generated/logs/transcript_full.jsonl', 'r') as f:
    for line in f:
        try:
            data = json.loads(line)
            if data.get('type') == 'PLANNER_RESPONSE':
                for call in data.get('tool_calls', []):
                    if call.get('name') in ('multi_replace_file_content', 'replace_file_content'):
                        args = call.get('args', {})
                        # parse if string
                        if isinstance(args, str):
                            args = json.loads(args)
                            
                        target = args.get('TargetFile', '')
                        if 'src/routes/' in target:
                            print(f"File: {target}")
                            print(f"Instruction: {args.get('Instruction')}")
                            if 'ReplacementChunks' in args:
                                for chunk in args['ReplacementChunks']:
                                    print("--- TARGET ---")
                                    print(chunk.get('TargetContent', '').strip())
                                    print("--- REPLACEMENT ---")
                                    print(chunk.get('ReplacementContent', '').strip())
                            else:
                                print("--- TARGET ---")
                                print(args.get('TargetContent', '').strip())
                                print("--- REPLACEMENT ---")
                                print(args.get('ReplacementContent', '').strip())
                            print("="*40)
        except Exception as e:
            pass

