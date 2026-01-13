import json
import subprocess
import os
import sys
from concurrent.futures import ThreadPoolExecutor

def get_espeak_ipa(words, voice):
    # Process words in a batch to speed up espeak execution
    cmd = ['espeak-ng', '-q', '--ipa=3', '-v', voice, " ".join(words)]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        # Espeak separates words with spaces in IPA output
        # Handle cases where multiple spaces might occur or espeak adds extra output
        ipas = result.stdout.strip().split()
        return ipas
    except Exception as e:
        print(f"Error processing batch with {voice}: {e}", file=sys.stderr)
        return []

def process_batch(batch, voice):
    ipas = get_espeak_ipa(batch, voice)
    results = {}
    
    if len(ipas) == len(batch):
        for word, ipa in zip(batch, ipas):
            results[word] = ipa
    else:
        # Fallback to one-by-one if batch size mismatch occurs
        for word in batch:
            ipa_list = get_espeak_ipa([word], voice)
            if ipa_list:
                results[word] = ipa_list[0]
                    
    return results

def process_dictionary(words, voice):
    all_results = {}
    
    print(f"Processing {len(words)} words for {voice} with 4x parallelism...")
    batch_size = 100
    batches = [words[i:i+batch_size] for i in range(0, len(words), batch_size)]
    
    with ThreadPoolExecutor(max_workers=4) as executor:
        futures = [executor.submit(process_batch, b, voice) for b in batches]
        
        for i, future in enumerate(futures):
            try:
                res = future.result()
                all_results.update(res)
            except Exception as e:
                print(f"Error in future result: {e}")
            
            if (i + 1) % 50 == 0:
                print(f"  Progress: {(i + 1) * batch_size}/{len(words)}")
            
    return all_results

def main():
    data_path = 'data.txt'
    if not os.path.exists(data_path):
        print(f"Error: {data_path} not found")
        return

    with open(data_path, 'r') as f:
        all_words = [line.strip() for line in f if line.strip()]

    # Load existing dictionaries
    def load_json(name):
        path = f'data/{name}.json'
        if os.path.exists(path):
            with open(path, 'r') as f:
                return json.load(f)
        return {}

    us_gold = load_json('us_gold')
    us_silver = load_json('us_silver')
    gb_gold = load_json('gb_gold')
    gb_silver = load_json('gb_silver')

    # Process US
    print("Processing US English...")
    us_results = process_dictionary(all_words, 'en-us')
    
    print("Updating US dictionaries (Gold: existing keys, Silver: others)...")
    for word, ipa in us_results.items():
        if word in us_gold:
            us_gold[word] = ipa
        else:
            us_silver[word] = ipa
    
    with open('data/us_gold.json', 'w') as f:
        json.dump(us_gold, f, ensure_ascii=False, indent=2, sort_keys=True)
    with open('data/us_silver.json', 'w') as f:
        json.dump(us_silver, f, ensure_ascii=False, indent=2, sort_keys=True)

    # Process GB
    print("Processing British English...")
    gb_results = process_dictionary(all_words, 'en-gb')
    
    print("Updating GB dictionaries (Gold: existing keys, Silver: others)...")
    for word, ipa in gb_results.items():
        if word in gb_gold:
            gb_gold[word] = ipa
        else:
            gb_silver[word] = ipa
    
    with open('data/gb_gold.json', 'w') as f:
        json.dump(gb_gold, f, ensure_ascii=False, indent=2, sort_keys=True)
    with open('data/gb_silver.json', 'w') as f:
        json.dump(gb_silver, f, ensure_ascii=False, indent=2, sort_keys=True)

    print("Successfully updated US and GB dictionaries. Gold keys preserved, new words in Silver.")

if __name__ == "__main__":
    main()
