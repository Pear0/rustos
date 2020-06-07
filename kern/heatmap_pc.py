import re
import numpy as np
import seaborn as sns
import matplotlib.pylab as plt
import collections


# 17468@1590719672.352466:guest_mem_before_exec cpu=0x7fc6200c5080 vaddr=0x00000000000000f0 info=515

parse_re = re.compile(r'[0-9]+@[0-9.]+:guest_mem_before_exec cpu=(0x[a-f0-9]+) vaddr=(0x[a-f0-9]+) info=[0-9]+', re.IGNORECASE)

values = []
with open('kernel_trace.log', 'r') as f:
    for line in f:
        try:
            match = parse_re.match(line)
            cpu_info, vaddr = eval(match.group(1)), eval(match.group(2))
            values.append(vaddr)
            # if len(values) == 1000:
            #     break
        except AttributeError:
            print('failed on ' + line)
            exit(1)

maximum = max(values)
width = 1024
counter = collections.Counter()

two_d = np.ones((1 + (maximum // width), width))
for value in values:
    two_d[value // width, value % width] += 1
    counter[value] += 1

print('Most Common:')
for addr, count in counter.most_common(20):
    print('addr', hex(addr), '=', count)

# two_d = np.log10(two_d)

ax = sns.heatmap(two_d)
plt.show()
