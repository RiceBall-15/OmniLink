#!/usr/bin/env python3
"""Fix task queue state: reset tasks with incomplete subtasks back to pending."""

import sys

TASK_FILE = '/root/omnilink/TASK_QUEUE.md'
CHECKMARK = '\u2705'  # check mark
HOURGLASS = '\u23f3'  # hourglass

with open(TASK_FILE, 'r') as f:
    lines = f.readlines()

in_task = False
task_line_idx = None
has_incomplete = False
fixed_count = 0

for i, line in enumerate(lines):
    stripped = line.strip()
    if stripped.startswith('####') and CHECKMARK in line:
        # Save previous task if it had incomplete subtasks
        if in_task and has_incomplete and task_line_idx is not None:
            lines[task_line_idx] = lines[task_line_idx].replace(CHECKMARK, HOURGLASS, 1)
            fixed_count += 1
        in_task = True
        task_line_idx = i
        has_incomplete = False
    elif stripped.startswith('####'):
        if in_task and has_incomplete and task_line_idx is not None:
            lines[task_line_idx] = lines[task_line_idx].replace(CHECKMARK, HOURGLASS, 1)
            fixed_count += 1
        in_task = False
        task_line_idx = None
    elif in_task and '- [ ]' in line:
        has_incomplete = True

# Handle last task
if in_task and has_incomplete and task_line_idx is not None:
    lines[task_line_idx] = lines[task_line_idx].replace(CHECKMARK, HOURGLASS, 1)
    fixed_count += 1

if fixed_count > 0:
    with open(TASK_FILE, 'w') as f:
        f.writelines(lines)
    print(f'Fixed {fixed_count} task states (reset to pending)')
else:
    print('No fixes needed - all tasks have correct state')
