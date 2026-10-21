import json, os, collections
CAP=50; RESERVE=10; BAND_CAP=10; COMP_CAP=50
REPOS={'tmux':'/home/shafin/Opencode/perfpal-test/out-i5-tmux','hyperfine':'/home/shafin/Opencode/perfpal-test/out-i5-hyperfine','cJSON':'/home/shafin/Opencode/perfpal-test/out-i5-cJSON','chunkhound':'/var/tmp/opencode/v1-out-chunkhound','ripgrep':'/var/tmp/opencode/v1-out-ripgrep','curl':'/var/tmp/opencode/v1-out-curl'}
TARGETS={'tmux':['input.c','screen-write.c','tmux.h'],'hyperfine':['src/benchmark/mod.rs','src/command.rs','src/timer/mod.rs'],'cJSON':['cJSON.c','cJSON.h'],'chunkhound':['src/db/duckdb_backend/write.rs','src/pipeline/pipeline.rs','src/db/duckdb_backend/compaction.rs'],'ripgrep':['crates/globset/src/lib.rs'],'curl':['lib/file.c']}
GENUINE=[('chunkhound','PERF-RS-ALLOC-001:src/db/duckdb_backend/write.rs:133:13'),('chunkhound','PERF-RS-ALLOC-001:src/db/duckdb_backend/write.rs:239:17'),('chunkhound','PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:922:29'),('chunkhound','PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:1431:25'),('chunkhound','PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:1493:9'),('chunkhound','PERF-RS-ARCH-002:src/db/duckdb_backend/compaction.rs:32:5'),('ripgrep','PERF-RS-ALLOC-001:crates/globset/src/lib.rs:424:9'),('curl','PERF-C-LOOP-002:lib/file.c:387:14'),('curl','PERF-C-LOOP-002:lib/file.c:582:15')]
def trunc(p): return p if len(p)<=32 else '…'+p[-31:]
def comp_of(path,comps):
    best=None
    for c in comps:
        p=c['path_prefix']
        if p=='.' or path==p or path.startswith(p+'/'):
            if best is None or len(p)>len(best['path_prefix']): best=c
    return best
def analyze(repo):
    d=json.load(open(REPOS[repo]+'/metrics.json'))
    cost=collections.defaultdict(float); entry=collections.defaultdict(bool); reach=collections.defaultdict(bool)
    for f in d['stats']['functions']:
        c=f.get('function_cost_v2'); c=f.get('function_cost',0) if c is None else c
        cost[f['path']]+=c or 0
        if f.get('reachable_from_entry'): reach[f['path']]=True
        ek=f.get('entry_kind')
        if ek and ek!='unknown-entry': entry[f['path']]=True
    nodes=sorted({n['path'] for n in d['graphs']['include_graph']['nodes']}); ns=set(nodes)
    pairs=[(e['from'],e['to']) for e in d['graphs']['include_graph']['edges'] if e.get('resolved')]
    deg=collections.Counter({p:0 for p in nodes})
    for a,b in pairs:
        if a in ns and b in ns: deg[a]+=1; deg[b]+=1
    ranked=sorted(deg.items(),key=lambda x:(-x[1],x[0])); deg_rank={p:i+1 for i,(p,_) in enumerate(ranked)}
    base=[p for p,_ in ranked[:CAP]]
    reserve=sorted([p for p,_ in ranked[CAP:] if reach.get(p)],key=lambda p:(-cost.get(p,0),p))[:RESERVE]
    shown_file=set(base)|set(reserve)
    cost_sorted=sorted(ns|set(cost),key=lambda p:(-cost.get(p,0),p)); cost_rank={p:i+1 for i,p in enumerate(cost_sorted)}
    sev={'hot':3,'warn':2,'info':1}; weight=collections.Counter(); fcount=collections.Counter()
    for f in d['findings']: weight[f['path']]+=sev.get(f['severity'],0); fcount[f['path']]+=1
    wsorted=sorted(ns|set(weight),key=lambda p:(-weight.get(p,0),p)); wrank={p:i+1 for i,p in enumerate(wsorted)}
    shown_find=set(wsorted[:CAP])
    entries=sorted([p for p in ns if entry.get(p)],key=lambda p:(-cost.get(p,0),p)); shown_entry=set(entries[:CAP])
    if len(entries)<CAP: shown_entry|=set([p for p in ns if not entry.get(p)][:CAP-len(entries)])
    comps=d['components']; band_shown=set(); band_rank={}
    for c in comps:
        if c['rank']>COMP_CAP: continue
        pref=c['path_prefix']
        members=[p for p in ns|set(f['path'] for f in d['files']) if (os.path.dirname(p) or '.')==pref]
        s=sorted(members,key=lambda p:(-cost.get(p,0),p))
        for i,p in enumerate(s[:BAND_CAP]): band_shown.add(p); band_rank[p]=(i+1,len(s))
    topcomps=sorted(comps,key=lambda c:c['rank'])[:10]
    top_find=set(); top_by_comp={}
    for c in topcomps:
        fs=[f for f in d['findings'] if (comp_of(f['path'],comps) or {}).get('component_id')==c['component_id']]
        fs=sorted(fs,key=lambda f:(-sev.get(f['severity'],0),f['finding_id']))[:3]
        top_find|={f['finding_id'] for f in fs}; top_by_comp[c['component_id']]=[f['finding_id'] for f in fs]
    return dict(d=d,cost=cost,entry=entry,reach=reach,deg=deg,deg_rank=deg_rank,cost_rank=cost_rank,
        weight=weight,fcount=fcount,wrank=wrank,shown_file=shown_file,shown_entry=shown_entry,
        shown_find=shown_find,band_shown=band_shown,band_rank=band_rank,comps=comps,topcomps=topcomps,
        top_find=top_find,top_by_comp=top_by_comp,nodes=nodes)
def describe(a,p):
    c=comp_of(p,a['comps'])
    return {'path':p,'label':trunc(p),'cost_sum':round(a['cost'].get(p,0)),'cost_rank':a['cost_rank'].get(p),'cost_total':len(a['cost_rank']),
      'degree':a['deg'].get(p,0),'degree_rank':a['deg_rank'].get(p),'degree_total':len(a['nodes']),
      'findings':a['fcount'].get(p,0),'weight':a['weight'].get(p,0),'weight_rank':a['wrank'].get(p),'weight_total':len(a['wrank']),
      'component':c['path_prefix'] if c else None,'component_rank':c['rank'] if c else None,
      'component_density':round(c['metrics']['finding_density'],2) if c else None,
      'shown_include_graph_layer_file':p in a['shown_file'],'shown_layer_entry':p in a['shown_entry'],
      'shown_layer_findings':p in a['shown_find'],'shown_layer_bands':p in a['band_shown'],
      'band_rank':a['band_rank'].get(p),'component_in_map_table':bool(c and c['component_id'] in {x['component_id'] for x in a['topcomps']}),
      'function_or_line_named':False,'is_entry_file':a['entry'].get(p,False)}
out={}
for repo,targets in TARGETS.items():
    a=analyze(repo); out[repo]={'targets':[describe(a,p) for p in targets],'genuine':[]}
    for r,fid in GENUINE:
        if r!=repo: continue
        path=fid.split(':',1)[1].rsplit(':',2)[0]; rec=describe(a,path); rec['finding_id']=fid
        f=[x for x in a['d']['findings'] if x['finding_id']==fid]
        rec['present']=bool(f); rec['in_map_table']=fid in a['top_find']
        if f: rec.update(symbol=f[0].get('symbol'),severity=f[0].get('severity'),confidence=f[0].get('confidence'))
        c=comp_of(path,a['comps'])
        rec['component_top3']=a['top_by_comp'].get(c['component_id'],[]) if c else []
        out[repo]['genuine'].append(rec)
json.dump(out,open('/var/tmp/opencode/recall2.json','w'),indent=1)
# compact print
for repo,v in out.items():
    print('##',repo)
    for r in v['targets']:
        print(f"  {r['path']:40} cost#{r['cost_rank']}/{r['cost_total']} deg#{r['degree_rank']}/{r['degree_total']} w#{r['weight_rank']} comp#{r['component_rank']} | file={r['shown_include_graph_layer_file']} entry={r['shown_layer_entry']} find={r['shown_layer_findings']} band={r['shown_layer_bands']} mapTab={r['component_in_map_table']} label='{r['label']}'")
    for r in v['genuine']:
        print(f"  GEN {r['finding_id']}")
        print(f"      file={r['shown_include_graph_layer_file']} entry={r['shown_layer_entry']} find={r['shown_layer_findings']} band={r['shown_layer_bands']} mapTable={r['in_map_table']} w#{r['weight_rank']}/{r['weight_total']} comp#{r['component_rank']} top3={r['component_top3']}")
