import pyphen
import sys

p = pyphen.Pyphen(lang="en")
yarn = sys.argv[1]
hlist = sys.argv[2]

hlist = open(hlist).readlines()
hlist = map(str.strip, hlist)
hlist = set(hlist)
existing = map(lambda w: w.replace("|", ""), hlist)
existing = set(existing)

words = open(yarn).read().split()
words = filter(str.isalpha, words)
words = map(str.lower, words)
words = set(words)

missing = words - existing
missing = map(lambda w: p.inserted(w, "|"), missing)
missing = filter(lambda w: "|" in w, missing)
missing = set(missing)

both = hlist | missing
print("\n".join(sorted(both)))
