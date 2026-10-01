"""Visible target for real remote screen/input acceptance; no RustDesk protocol code."""
import json
from pathlib import Path
import socket
import time
import tkinter as tk

root = tk.Tk()
root.title('Goal remote input probe')
root.geometry('500x280+60+80')
events = []
destination = Path.home() / 'evidence' / 'remote-input.json'
tk.Label(root, text=socket.gethostname(), font=('sans', 20)).pack()
clock = tk.Label(root, font=('sans', 24))
clock.pack()
tk.Label(root, text='Type a unique marker through the remote session:').pack()
entry = tk.Entry(root, font=('sans', 16))
entry.pack(fill='x', padx=20)

def record(kind):
    events.append({'kind': kind, 'time': time.time(), 'text': entry.get()})
    destination.write_text(json.dumps(events, indent=2))

entry.bind('<KeyRelease>', lambda event: record('keyboard'))
button = tk.Button(root, text='Record remote mouse click', command=lambda: record('mouse'))
button.pack(pady=15)

def tick():
    current = int(time.time())
    clock.configure(text=str(current), bg=['lightblue', 'lightgreen'][current % 2])
    root.after(500, tick)

tick()
root.mainloop()
