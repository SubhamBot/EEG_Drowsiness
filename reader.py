import pandas as pd
import serial
import time
import threading
from pynput import keyboard

# --- Configuration ---
COM_PORT = 'COM6' 
BAUD_RATE = 115200
FILE_NAME = 'acquiredDataset.csv'
SAMPLING_INTERVAL = 0.1 

# --- Simulation State ---
state = {
    "speed": 60.0,
    "running": True
}

# --- Serial Initialization ---
try:
    # Added write_timeout=0.05 to prevent infinite blocking, 
    # but still allow try/except to catch it.
    ser = serial.Serial(COM_PORT, BAUD_RATE, timeout=0.1, write_timeout=0.05)
    print(f"Connected to {COM_PORT}")
except Exception as e:
    print(f"Serial Error: {e}")
    exit()

def on_press(key):
    try:
        if key == keyboard.Key.up:
            state["speed"] += 5.0
        elif key == keyboard.Key.down:
            state["speed"] = max(0.0, state["speed"] - 5.0)
        
        # Wrapped keyboard-driven write
        payload = f"S,{state['speed']:.1f}\n"
        try:
            ser.write(payload.encode())
        except serial.SerialTimeoutException:
            pass # Ignore timeout on manual input to keep UI responsive
            
    except Exception:
        pass

# Background Keyboard Listener
listener = keyboard.Listener(on_press=on_press)
listener.start()

def stream_data():
    df = pd.read_csv(FILE_NAME)
    print("Streaming started. Use UP/DOWN arrows to simulate speed.")
    
    for _, row in df.iterrows():
        if not state["running"]:
            break
        
        alpha = row['lowAlpha'] + row['highAlpha']
        beta = row['lowBeta'] + row['highBeta']
        
        # EEG Packet: E,Alpha,Beta
        payload = f"E,{alpha:.1f},{beta:.1f}\n"
        
        try:
            ser.write(payload.encode())
        except serial.SerialTimeoutException:
            # Handle the write timeout gracefully
            print(f"\n[Warning] Write Timeout: STM32 buffer full. Dropping packet.", end='')
        
        print(f"Current Speed: {state['speed']:.1f} km/h | Tx: {payload.strip()}    ", end='\r')
        time.sleep(SAMPLING_INTERVAL)

try:
    stream_data()
except KeyboardInterrupt:
    state["running"] = False
    print("\nSimulation Stopped.")