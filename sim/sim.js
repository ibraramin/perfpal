/* Atlas run-story prototype (Track D, Session D1).
 *
 * Replays the shipped Atlas entry-flow map from Perfpal's atlas-layout.json
 * projection (`data-<case>.json`) as a deterministic, scrubbable,
 * static-inferred story.
 * No randomness, no wall-clock input beyond the animation clock, no network.
 */
(() => {
  'use strict';

  const SVG = 'http://www.w3.org/2000/svg';
  const DURATION = {
    boot: 1600,
    enter: 2200,
    loop: 2600,
    branch: 1800,
    hottest: 2000,
    profile: 2400,
    'start-here': 2600,
    stop: 2000,
    trust: 2600,
    summary: 3000,
  };
  const SEVERITY = { hot: 0, warn: 1, info: 2 };
  const SPEEDS = { 1: 0.5, 2: 1, 3: 2, 4: 4 };

  const state = {
    cases: [],
    caseId: null,
    data: null,
    byId: new Map(),
    beats: [],
    index: 0,
    ms: 0,
    totalMs: 0,
    playing: false,
    speed: 1,
    selected: null,
    synthetic: false,
    reduced: false,
    appliedIndex: -1,
    appliedSelected: null,
  };

  const dom = { nodes: new Map(), edges: [], edgeLabels: [], nodeTop: new Map() };
  let frame = null;
  let lastTick = 0;

  const $ = (selector) => document.querySelector(selector);

  function svg(tag, attrs = {}, text) {
    const node = document.createElementNS(SVG, tag);
    for (const [key, value] of Object.entries(attrs)) {
      if (value !== null && value !== undefined && value !== '') node.setAttribute(key, String(value));
    }
    if (text !== undefined) node.textContent = text;
    return node;
  }

  function html(tag, attrs = {}, text) {
    const node = document.createElement(tag);
    for (const [key, value] of Object.entries(attrs)) {
      if (key === 'class') node.className = value;
      else if (key === 'text') node.textContent = value;
      else if (value !== null && value !== undefined) node.setAttribute(key, String(value));
    }
    if (text !== undefined) node.textContent = text;
    return node;
  }

  const clamp = (value, low, high) => Math.max(low, Math.min(high, value));
  const fmt = (value) => Number(value).toLocaleString('en-US');
  const base = (path) => String(path).split('/').pop();

  function compact(value) {
    const number = Number(value);
    if (!Number.isFinite(number)) return String(value);
    if (number >= 1e6) return `${(number / 1e6).toFixed(1)}M`;
    if (number >= 1e4) return `${(number / 1e3).toFixed(0)}k`;
    if (number >= 1e3) return `${(number / 1e3).toFixed(1)}k`;
    return String(number);
  }

  function fitted(text, perChar, max = 138) {
    const width = String(text).length * perChar;
    return width > max ? { textLength: max, lengthAdjust: 'spacingAndGlyphs' } : {};
  }

  function midEllipsis(value, max = 30) {
    const text = String(value);
    if (text.length <= max) return text;
    const head = Math.ceil((max - 1) / 2);
    const tail = Math.floor((max - 1) / 2);
    return `${text.slice(0, head)}…${text.slice(-tail)}`;
  }

  function setText(id, value) {
    const node = $(id);
    if (node) node.textContent = value === null || value === undefined ? '' : String(value);
  }

  /* ---------------------------------------------------------------- beats */

  function loopFunction(node) {
    const functions = [...node.functions]
      .filter((fn) => !fn.module && (fn.maxLoopDepth > 0 || fn.unknownTripLoops > 0))
      .sort((a, b) => b.cost - a.cost || a.line - b.line);
    return functions[0] || null;
  }

  function topFinding(data, path) {
    return [...data.findings]
      .filter((finding) => finding.path === path)
      .sort((a, b) => (SEVERITY[a.severity] ?? 9) - (SEVERITY[b.severity] ?? 9) || a.line - b.line)[0] || null;
  }

  function stateChip(data) {
    if (data.state.empty) return 'empty';
    if (data.state.noEntry) return 'no entry';
    if (data.state.library) return 'no forward path';
    if (data.state.allTest) return 'test-dominated';
    if (data.state.capped) return 'capped';
    if (data.state.partialParse) return 'partial parse';
    return 'static leaf';
  }

  function stopText(data) {
    const parts = [];
    if (data.state.library) {
      parts.push('no forward path: the elected entry is a library root, so callers are reverse context, not a run');
    } else if (data.state.empty || data.state.noEntry) {
      parts.push('no graph to walk: this is a named stub state');
    } else if (data.state.capped) {
      parts.push('the story stops at the canvas cap, not at a leaf; the real page draws top-cost files per depth plus aggregated boxes');
    } else {
      parts.push('the representative path ends at a static leaf (an inferred end, not a runtime return)');
    }
    if (data.state.partialParse) parts.push('the scan is partially parsed, so some structure may be missing');
    if (data.state.allTest) parts.push('the scan is test/bench-dominated');
    return `${parts.join('; ')}.`;
  }

  function summaryText(data) {
    const total = data.meta.findingsTotal ?? data.findings.length;
    const included = data.meta.findingsIncluded ?? data.findings.length;
    const findings = data.synthetic
      ? 'no findings in this state demo'
      : included < total
        ? `${fmt(total)} findings in the scan (${fmt(included)} included in this data slice)`
        : `${fmt(total)} findings in the scan`;
    return `${data.meta.path} · ${data.meta.caps} · ${findings}. Static cost units, not time; profile to confirm.`;
  }

  function profileText(data) {
    const rows = (data.profile || []).slice(0, 5);
    if (rows.length === 0) return 'No drawn-slice static candidates recorded for this data slice.';
    return `Top 3 per file shown; static candidates, verify: ${rows
      .map((fn) => `${fn.name} (${base(fn.path)} L${fn.line}) · ${fmt(fn.cost)} units`)
      .join(' · ')}. Drawn-slice queue, not a whole-repo ranking.`;
  }

  function anchorPath(anchor) {
    return String(anchor).replace(/:\d+$/, '');
  }

  function startHereText(data) {
    const actions = (data.meta.actions || []).slice(0, 3);
    if (actions.length === 0) return 'No ranked actions recorded for this scan.';
    return `${actions
      .map((action) => `${action.anchor} — ${action.title} (${action.onCanvas ? 'on-canvas' : 'off-canvas'})`)
      .join(' · ')}.`;
  }

  function trustText(data) {
    const trust = data.trust;
    if (!trust) return 'Trust: cap, parse and coverage accounting is not present in this data slice.';
    return [
      `caps: ${trust.caps}`,
      `partial parse: ${trust.partialParse ? 'yes' : 'no'}`,
      `non-production share: ${fmt(trust.nonProductionNodes)}/${fmt(trust.drawnNodes)} drawn files (test/bench)`,
      `unknown-trip functions: ${fmt(trust.unknownTripFunctions)} (estimate incomplete)`,
      `findings included: ${fmt(trust.findingsIncluded)}/${fmt(trust.findingsTotal)}`,
      ioTrustText(trust.io),
    ].filter(Boolean).join(' · ') + '.';
  }

  // S3: I/O coverage is a fixed-table call-name match. A missing literal is an
  // unknown argument target, never an invented path; stdio has no path target.
  function ioTrustText(io) {
    if (!io) return '';
    if (!io.rows) return 'I/O: no fixed-table call-name match in the drawn files';
    const examples = (io.literals || [])
      .slice(0, 2)
      .map((row) => `${row.target} "${midEllipsis(row.literal, 16)}"`);
    const more = io.literal > examples.length ? `, +${io.literal - examples.length}` : '';
    const literal = `${fmt(io.literal)} literal${examples.length ? ` (${examples.join(', ')}${more})` : ''}`;
    const stdio = io.stdio ? `, ${fmt(io.stdio)} stdio` : '';
    return `I/O: ${fmt(io.rows)} effect${io.rows === 1 ? '' : 's'} in ${fmt(io.files)} file${io.files === 1 ? '' : 's'} — ${literal}, ${fmt(io.unknownTarget)} target unknown${stdio}; call-name match`;
  }

  function storesText(node) {
    const io = node.io;
    if (!io || !io.total) return '';
    const counts = [];
    if (io.read) counts.push(`${fmt(io.read)} read${io.read === 1 ? '' : 's'}`);
    if (io.write) counts.push(`${fmt(io.write)} write${io.write === 1 ? '' : 's'}`);
    const examples = io.rows
      .filter((row) => row.literal !== null)
      .slice(0, 2)
      .map((row) => `${row.target} "${midEllipsis(row.literal, 18)}"`);
    const unknown = io.unknownTarget
      ? `${fmt(io.unknownTarget)} target${io.unknownTarget === 1 ? '' : 's'} unknown`
      : '';
    const stdio = io.stdio ? `${fmt(io.stdio)} stdio` : '';
    const detail = [...examples, unknown, stdio].filter(Boolean).join(' · ');
    return ` Stores: ${counts.join(' + ')}${detail ? ` — ${detail}` : ''}.`;
  }

  function buildBeats(data) {
    const beats = [];
    const outgoing = new Map();
    for (const edge of data.edges) {
      if (!outgoing.has(edge.from)) outgoing.set(edge.from, []);
      outgoing.get(edge.from).push(edge);
    }
    const byId = new Map(data.nodes.map((node) => [node.id, node]));

    if (data.nodes.length === 0) {
      const stub = data.meta.stub || { title: 'no graph', body: 'This state has no canvas nodes.' };
      beats.push({
        kind: 'boot',
        title: 'Boot · state demo',
        narration: `${stub.title}. ${stub.body}`,
        chips: ['synthetic', 'inferred'],
        step: 0,
        focus: {},
      });
      beats.push({
        kind: 'stop',
        title: `Stop · ${stateChip(data)}`,
        narration: stopText(data),
        chips: ['synthetic', 'no runtime'],
        step: 0,
        focus: {},
      });
      beats.push({
        kind: 'summary',
        title: 'Summary · state demo',
        narration: summaryText(data),
        chips: ['synthetic', 'no numbers fabricated'],
        step: 0,
        focus: {},
      });
      return beats;
    }

    const trace = [...data.trace].sort((a, b) => a.step - b.step);
    const entry = byId.get(trace[0].path) || data.nodes[0];
    beats.push({
      kind: 'boot',
      title: `Boot · ${base(entry.name)}`,
      narration: `${data.meta.entry}. The story follows the representative path (static, BFS-derived); nothing is executed.`,
      chips: ['inferred', 'entry'],
      step: 0,
      focus: { node: entry.id },
    });

    let previous = null;
    for (const step of trace) {
      const node = byId.get(step.path);
      if (!node) continue;
      const from = previous ? ` from ${base(previous)}` : '';
      beats.push({
        kind: 'enter',
        title: `Step ${step.step}/${trace.length} · ${base(node.name)}`,
        narration: `${node.costText}${from}${node.ratioText ? ` · ${node.ratioText}` : ''}${node.topText ? ` · ${node.topText}` : ''}.${node.badge ? ` Annotation ${node.badge}.` : ''}${storesText(node)}`,
        chips: [`step ${step.step}/${trace.length}`, 'inferred'],
        step: step.step,
        focus: { node: node.id, edge: previous ? `${previous}->${node.id}` : null },
      });

      const loop = loopFunction(node);
      if (loop) {
        beats.push({
          kind: 'loop',
          title: `Loop moment · ${loop.name}`,
          narration: `${loop.name} (line ${loop.line}) carries ${fmt(loop.cost)} static cost units · max loop depth ${loop.maxLoopDepth} · ${loop.unknownTripLoops} unknown-trip loop${loop.unknownTripLoops === 1 ? '' : 's'} · trip provenance ${loop.tripSource || 'unknown'}. Inferred trip weight, not measured iterations.`,
          chips: ['inferred', `trip ${loop.tripSource || 'unknown'}`],
          step: step.step,
          focus: { node: node.id, fn: loop.id },
        });
      }

      const branches = (outgoing.get(node.id) || []).filter((edge) => !edge.onPath);
      if (branches.length > 0) {
        const largest = [...branches]
          .sort((a, b) => b.count - a.count || a.to.localeCompare(b.to))
          .slice(0, 2);
        const detail = largest.map((edge) => `${base(edge.to)} ×${edge.count}`).join(', ');
        beats.push({
          kind: 'branch',
          title: `Branch point · ${branches.length} off-path edge${branches.length === 1 ? '' : 's'}`,
          narration: `${branches.length} structural call edge${branches.length === 1 ? ' leaves' : 's leave'} ${base(node.name)} away from the representative path${detail ? ` (largest: ${detail})` : ''}. The story does not follow them; they are not measured.`,
          chips: ['inferred', 'not taken'],
          step: step.step,
          focus: { node: node.id },
        });
      }
      previous = node.id;
    }

    const hottest = data.nodes.find((node) => node.hottest);
    if (hottest) {
      const finding = topFinding(data, hottest.id);
      const hottestFunction = data.hottestFunction || (hottest.functions || [])[0] || null;
      const fileParts = [hottest.costText || 'cost unavailable'];
      if (hottest.ratioText) fileParts.push(hottest.ratioText);
      if (hottest.topText) fileParts.push(hottest.topText);
      const narration = [`Hottest file: ${base(hottest.name)} · ${fileParts.join(' · ')}.`];
      if (hottestFunction) {
        narration.push(
          `Hottest function: ${hottestFunction.name} (L${hottestFunction.line}, ${hottestFunction.path}) · ${fmt(hottestFunction.cost)} units.`,
        );
      }
      if (finding) {
        narration.push(
          `Top annotation: ${finding.severity.toUpperCase()} ${finding.ruleId} — ${finding.title} (line ${finding.line}).`,
        );
      }
      beats.push({
        kind: 'hottest',
        title: `Hottest · ${base(hottest.name)}`,
        narration: narration.join(' '),
        chips: ['inferred', 'rank 1 by cost'],
        step: trace.length,
        focus: hottestFunction
          ? { node: hottestFunction.path, fn: hottestFunction.id }
          : { node: hottest.id },
      });
    }

    const profile = (data.profile || []).slice(0, 5);
    beats.push({
      kind: 'profile',
      title: profile.length > 0 ? 'Profile queue · drawn-slice candidates' : 'Profile queue',
      narration: profileText(data),
      chips: ['static candidate', 'verify'],
      step: trace.length,
      focus: profile.length > 0 ? { node: profile[0].path, fn: profile[0].id } : {},
    });

    const actions = (data.meta.actions || []).slice(0, 3);
    const onCanvasAction = actions.find(
      (action) => action.onCanvas && byId.has(anchorPath(action.anchor)),
    );
    beats.push({
      kind: 'start-here',
      title: `Start here · ${actions.length} action${actions.length === 1 ? '' : 's'}`,
      narration: startHereText(data),
      chips: ['ranked action', 'on/off canvas'],
      step: trace.length,
      focus: onCanvasAction ? { node: anchorPath(onCanvasAction.anchor) } : {},
    });

    beats.push({
      kind: 'stop',
      title: `Stop · ${stateChip(data)}`,
      narration: stopText(data),
      chips: ['inferred', stateChip(data)],
      step: trace.length,
      focus: { node: previous },
    });
    beats.push({
      kind: 'trust',
      title: 'Trust · what this slice covers',
      narration: trustText(data),
      chips: ['caps', 'coverage'],
      step: trace.length,
      focus: {},
    });
    beats.push({
      kind: 'summary',
      title: 'Summary',
      narration: summaryText(data),
      chips: ['inferred', 'static cost units (v2)'],
      step: trace.length,
      focus: {},
    });
    return beats;
  }

  function beatStart(index) {
    let start = 0;
    for (let i = 0; i < index; i += 1) start += DURATION[state.beats[i].kind] || 2000;
    return start;
  }

  /* --------------------------------------------------------------- render */

  function pathValues(pathData) {
    const numbers = String(pathData).match(/-?\d+(?:\.\d+)?/g) || [];
    return numbers.map(Number);
  }

  function cubicAt(values, t) {
    if (values.length === 8) {
      const [x0, y0, x1, y1, x2, y2, x3, y3] = values;
      const u = 1 - t;
      return {
        x: u * u * u * x0 + 3 * u * u * t * x1 + 3 * u * t * t * x2 + t * t * t * x3,
        y: u * u * u * y0 + 3 * u * u * t * y1 + 3 * u * t * t * y2 + t * t * t * y3,
      };
    }
    if (values.length >= 4) {
      const x0 = values[0];
      const y0 = values[1];
      const x1 = values[values.length - 2];
      const y1 = values[values.length - 1];
      return { x: x0 + (x1 - x0) * t, y: y0 + (y1 - y0) * t };
    }
    return { x: 0, y: 0 };
  }

  function cubicTangent(values, t) {
    if (values.length === 8) {
      const [x0, y0, x1, y1, x2, y2, x3, y3] = values;
      const u = 1 - t;
      return {
        x: 3 * u * u * (x1 - x0) + 6 * u * t * (x2 - x1) + 3 * t * t * (x3 - x2),
        y: 3 * u * u * (y1 - y0) + 6 * u * t * (y2 - y1) + 3 * t * t * (y3 - y2),
      };
    }
    if (values.length >= 4) {
      return { x: values[values.length - 2] - values[0], y: values[values.length - 1] - values[1] };
    }
    return { x: 0, y: 1 };
  }

  /* ---------------------------------------------------- edge label layout */

  const LABEL_TS = [0.5, 0.38, 0.62, 0.28, 0.72];
  const LABEL_OFFSETS = [0, -8, 8, -14, 14];
  const LABEL_CHAR_W = 4.4;
  const LABEL_MAX_W = 150;

  const rect = (x1, y1, x2, y2) => ({ x1, y1, x2, y2 });
  const inflateRect = (box, pad) =>
    rect(box.x1 - pad, box.y1 - pad, box.x2 + pad, box.y2 + pad);
  const nodeRect = (node, pad = 0) =>
    rect(node.x - pad, node.y - pad, node.x + node.w + pad, node.y + node.h + pad);

  function overlapArea(a, b) {
    const w = Math.min(a.x2, b.x2) - Math.max(a.x1, b.x1);
    const h = Math.min(a.y2, b.y2) - Math.max(a.y1, b.y1);
    return w > 0 && h > 0 ? w * h : 0;
  }

  function pointInRect(x, y, box) {
    return x >= box.x1 && x <= box.x2 && y >= box.y1 && y <= box.y2;
  }

  function isDarkFill(fill) {
    const match = /^#([0-9a-f]{6})$/i.exec(fill || '');
    if (!match) return false;
    const value = parseInt(match[1], 16);
    const red = (value >> 16) & 255;
    const green = (value >> 8) & 255;
    const blue = value & 255;
    return (0.2126 * red + 0.7152 * green + 0.0722 * blue) / 255 < 0.42;
  }

  function nodeTextRect(node) {
    return rect(node.x + 10, node.y + 14, node.x + node.w - 10, node.y + node.h - 8);
  }

  function nodeBadgeRect(node) {
    if (!node.badge) return null;
    const width = Math.min(node.badge.length * 4.3, node.w - 16);
    return rect(node.x + node.w - 6 - width, node.y + 2, node.x + node.w - 6, node.y + 12);
  }

  function hullLabelRect(hull) {
    const width = hull.label.length * 5.4;
    return rect(hull.x + 8, hull.y + 2, hull.x + 8 + width, hull.y + 14);
  }

  function labelWidth(text) {
    return Math.min(String(text).length * LABEL_CHAR_W, LABEL_MAX_W);
  }

  function labelRect(x, y, text, anchor = 'middle') {
    const width = labelWidth(text);
    let x1;
    if (anchor === 'start') x1 = x;
    else if (anchor === 'end') x1 = x - width;
    else x1 = x - width / 2;
    return rect(x1, y - 8, x1 + width, y + 2);
  }

  function pathLabelCandidates(values) {
    const candidates = [];
    for (const t of LABEL_TS) {
      const point = cubicAt(values, t);
      const tangent = cubicTangent(values, t);
      const length = Math.hypot(tangent.x, tangent.y) || 1;
      const nx = -tangent.y / length;
      const ny = tangent.x / length;
      for (const offset of LABEL_OFFSETS) {
        candidates.push({ x: point.x + nx * offset, y: point.y + ny * offset, anchor: 'middle', source: 'path' });
      }
    }
    return candidates;
  }

  function gridLabelCandidates(origin) {
    const candidates = [];
    for (const dy of [0, 10, -10, 20, -20, 30, -30, 40, -40, 50, -50, 60, -60, 70, -70,
      80, -80, 90, -90, 100, -100, 110, -110, 120, -120]) {
      for (let dx = -320; dx <= 320; dx += 8) {
        candidates.push({ x: origin.x + dx, y: origin.y + dy, anchor: 'middle', source: 'grid' });
      }
    }
    candidates.sort((a, b) =>
      (Math.hypot(a.x - origin.x, a.y - origin.y) - Math.hypot(b.x - origin.x, b.y - origin.y))
      || (a.y - b.y) || (a.x - b.x));
    return candidates;
  }

  function marginLabelCandidates(origin, text, stage) {
    const half = labelWidth(text) / 2 + 6;
    const candidates = [];
    for (const y of [stage.h - 6, 58]) {
      for (let x = half; x <= stage.w - half; x += 12) {
        candidates.push({ x, y, anchor: 'middle', source: 'margin' });
      }
    }
    candidates.sort((a, b) =>
      (Math.abs(a.x - origin.x) - Math.abs(b.x - origin.x)) || (a.y - b.y));
    return candidates;
  }

  function scoreLabel(candidate, text, nodes, hullLabels, placed) {
    const box = labelRect(candidate.x, candidate.y, text, candidate.anchor);
    let nodeOverlap = 0;
    let textOverlap = 0;
    let hullOverlap = 0;
    let labelOverlap = 0;
    let anchorInside = false;
    let darkOverlap = false;
    for (const node of nodes) {
      if (pointInRect(candidate.x, candidate.y, nodeRect(node, 1))) anchorInside = true;
      const cardOverlap = overlapArea(box, nodeRect(node, 4));
      if (cardOverlap > 0) {
        nodeOverlap += cardOverlap;
        if (isDarkFill(node.fill)) darkOverlap = true;
      }
      textOverlap += overlapArea(box, nodeTextRect(node));
      const badge = nodeBadgeRect(node);
      if (badge) textOverlap += overlapArea(box, badge);
    }
    for (const hull of hullLabels) hullOverlap += overlapArea(box, hull);
    for (const other of placed) labelOverlap += overlapArea(box, other);
    return {
      score: nodeOverlap + textOverlap * 8 + hullOverlap * 8 + labelOverlap * 12,
      nodeOverlap,
      textOverlap,
      hullOverlap,
      labelOverlap,
      clean: !anchorInside && !darkOverlap && nodeOverlap === 0
        && textOverlap === 0 && hullOverlap === 0 && labelOverlap === 0,
      hard: anchorInside || darkOverlap,
      box,
    };
  }

  function placeEdgeLabel(edge, label, context, placed) {
    const values = pathValues(edge.d);
    const origin = cubicAt(values, 0.5);
    const fullText = edge.payload ? `${edge.payload} · ${label}` : label;
    const attempts = [
      { text: fullText, dropped: false, list: pathLabelCandidates(values) },
      { text: label, dropped: Boolean(edge.payload), list: pathLabelCandidates(values) },
      { text: fullText, dropped: false, list: gridLabelCandidates(origin) },
      { text: label, dropped: Boolean(edge.payload), list: gridLabelCandidates(origin) },
      { text: fullText, dropped: false, list: marginLabelCandidates(origin, fullText, context.stage) },
      { text: label, dropped: Boolean(edge.payload), list: marginLabelCandidates(origin, label, context.stage) },
    ];
    const scored = [];
    for (const attempt of attempts) {
      for (const candidate of attempt.list) {
        const result = scoreLabel(candidate, attempt.text, context.nodes, context.hullLabels, placed);
        scored.push({ ...candidate, text: attempt.text, dropped: attempt.dropped, ...result });
      }
    }
    const byScore = (a, b) => a.score - b.score;
    const clean = scored.find((candidate) => candidate.clean);
    if (clean) return clean;
    const safeClass = scored.filter(
      (candidate) => candidate.dropped && !candidate.hard && candidate.nodeOverlap === 0,
    ).sort(byScore)[0];
    if (safeClass) return safeClass;
    const safe = scored.find((candidate) => !candidate.hard && candidate.nodeOverlap === 0);
    if (safe) return { ...safe, text: label, dropped: Boolean(edge.payload) };
    return scored.filter((candidate) => candidate.dropped).sort(byScore)[0] || scored[0];
  }


  function renderMeta() {
    const meta = state.data.meta;
    setText('#key-entry', meta.entry);
    setText('#key-caps', meta.caps);
    const hottest = (state.data.nodes || []).find((node) => node.hottest);
    if (hottest) {
      const ratioText = hottest.ratioText || '';
      const ratio = ratioText.match(/^([\d,]+(?:\.\d+)?)/);
      const ratioValue = ratio
        ? `${compact(Number(ratio[1].replace(/,/g, '')))}× median`
        : (ratioText || 'ratio unavailable');
      setText('#key-hottest-path', midEllipsis(hottest.name, 30));
      setText('#key-hottest-meta', `· ${compact(hottest.cost)} cost · ${ratioValue}`);
      $('#key-hottest').title = meta.hottest;
    } else {
      setText('#key-hottest-path', meta.hottest);
      setText('#key-hottest-meta', '');
    }
    setText('#key-path', meta.path);
    setText('#canvas-footer', meta.footer);

    const starts = $('#start-actions');
    starts.replaceChildren();
    if (!meta.actions || meta.actions.length === 0) {
      starts.textContent = 'no ranked actions recorded';
    } else {
      for (const action of meta.actions) {
        const span = html('span', { class: 'start' });
        span.append(html('span', { class: 'start-anchor' }, action.anchor));
        span.append(document.createTextNode(` — ${action.title}`));
        starts.append(span);
      }
    }
    $('#synthetic-chip').hidden = !state.synthetic;
  }

  function renderGraph() {
    const data = state.data;
    const stage = $('#stage');
    stage.replaceChildren();
    dom.nodes.clear();
    dom.edges = [];
    dom.edgeLabels = [];
    dom.nodeTop.clear();
    if (data.nodes.length === 0) return;

    stage.setAttribute('viewBox', `0 0 ${data.stage.w} ${data.stage.h}`);
    const defs = svg('defs');
    for (const [id, color] of [['sim-arrow', '#8d8577'], ['sim-arrow-path', '#c2410c']]) {
      const marker = svg('marker', {
        id, viewBox: '0 0 8 8', refX: 7, refY: 4, markerWidth: 6, markerHeight: 6, orient: 'auto',
      });
      marker.append(svg('path', { d: 'M0,0 L8,4 L0,8 Z', fill: color }));
      defs.append(marker);
    }
    stage.append(defs);

    for (const hull of data.hulls) {
      const group = svg('g', { class: 'hull' });
      group.append(svg('rect', { x: hull.x, y: hull.y, width: hull.w, height: hull.h, rx: 10 }));
      group.append(svg('text', { x: hull.x + 8, y: hull.y + 12 }, hull.label));
      stage.append(group);
    }

    // The renderer emits an UNREACHED lane even when no unreached file is drawn
    // (hyperfine's unreached production files are capped out). Draw the header only
    // when nodes actually sit in that lane.
    const hasNodes = (index, column) =>
      !column.unreached || data.nodes.some((node) => node.column === index);
    for (const [index, column] of data.columns.entries()) {
      if (!hasNodes(index, column)) continue;
      const group = svg('g', { class: 'column' });
      group.append(svg('text', {
        class: 'col-label', x: column.x, y: 22, 'text-anchor': 'middle',
      }, column.label));
      group.append(svg('text', {
        class: 'col-count', x: column.x, y: 36, 'text-anchor': 'middle',
      }, column.countText));
      stage.append(group);
    }

    const stepIndex = new Map(data.trace.map((step) => [step.path, step.step]));
    const centres = [...data.trace]
      .sort((a, b) => a.step - b.step)
      .map((step) => data.nodes.find((node) => node.id === step.path))
      .filter(Boolean)
      .map((node) => ({ x: node.x + node.w / 2, y: node.y + node.h / 2 }));
    if (centres.length > 1) {
      const d = centres.map((point, index) => `${index === 0 ? 'M' : 'L'} ${point.x} ${point.y}`).join(' ');
      stage.append(svg('path', { class: 'trace-line', d }));
    }

    for (const edge of data.edges) {
      const unresolved = edge.label === 'unresolved' || edge.labelClass === 'unresolved';
      const label = edge.label || 'inferred';
      const path = svg('path', {
        class: `edge${edge.back ? ' back' : ''}${edge.onPath ? ' onpath' : ''}${unresolved ? ' edge-unresolved' : ''}`,
        d: edge.d,
        'data-edge': `${edge.from}->${edge.to}`,
        'marker-end': `url(#sim-arrow${edge.onPath && !unresolved ? '-path' : ''})`,
      });
      const title = [
        `${edge.from} → ${edge.to}`,
        label,
        edge.payload || 'no payload',
        `×${edge.count}`,
      ];
      const top = (edge.topCalls || [])[0];
      if (top) title.push(`${top.callee} L${top.line}`);
      path.append(svg('title', {}, title.join(' · ')));
      stage.append(path);
      dom.edges.push({
        el: path,
        data: edge,
        step: stepIndex.get(edge.from) ?? Number.POSITIVE_INFINITY,
      });
    }

    for (const node of data.nodes) {
      const group = svg('g', {
        class: `node${node.entry ? ' is-entry' : ''}${node.hottest ? ' is-hottest' : ''}${node.nonproduction ? ' is-nonproduction' : ''}`,
        'data-node': node.id,
        role: 'button',
        tabindex: '0',
        'aria-label': `${node.id} · ${fmt(node.cost)} static cost units (v2) · ${node.ratioText}`,
      });
      group.append(svg('rect', {
        class: 'node-halo',
        x: node.x - 5,
        y: node.y - 5,
        width: node.w + 10,
        height: node.h + 10,
        rx: 10,
      }));
      group.append(svg('rect', {
        class: 'node-box',
        x: node.x,
        y: node.y,
        width: node.w,
        height: node.h,
        rx: 7,
        fill: node.fill,
        stroke: node.stroke,
        'stroke-width': node.strokeWidth,
        'stroke-dasharray': node.dash,
      }));
      group.append(svg('title', {}, node.badge ? `${node.title} · ${node.badge}` : node.title));
      if (node.badge) {
        // Inside the node's top-right, with a fill-matched halo so a lane
        // boundary never strikes through the text (the cJSON overprint).
        // Long badges condense to the node width instead of bleeding into a
        // neighbouring lane; the untruncated text stays in the node title.
        group.append(svg('text', {
          class: 'node-badge',
          x: node.x + node.w - 6,
          y: node.y + 11,
          'text-anchor': 'end',
          stroke: node.fill,
          ...fitted(node.badge, 4.3, node.w - 16),
        }, node.badge));
      }
      const name = midEllipsis(node.name, 24);
      group.append(svg('text', {
        class: 'node-name', x: node.x + node.w / 2, y: node.y + 26, 'text-anchor': 'middle',
        ...fitted(name, 6.2),
      }, name));
      group.append(svg('text', {
        class: 'node-cost', x: node.x + node.w / 2, y: node.y + 41, 'text-anchor': 'middle',
        ...fitted(node.costText, 5.4),
      }, node.costText));
      group.append(svg('text', {
        class: 'node-ratio', x: node.x + node.w / 2, y: node.y + 56, 'text-anchor': 'middle',
        ...fitted(node.ratioText, 5.4),
      }, node.ratioText));
      const top = svg('text', {
        class: 'node-top', x: node.x + node.w / 2, y: node.y + 71, 'text-anchor': 'middle',
      }, node.topText);
      group.append(top);
      dom.nodeTop.set(node.id, { el: top, text: node.topText });
      group.append(svg('circle', {
        class: 'path-dot', cx: node.x + 9, cy: node.y + 9, r: 7.5,
      }));
      if (node.pathStep) {
        group.append(svg('text', {
          class: 'path-num', x: node.x + 9, y: node.y + 12,
        }, String(node.pathStep)));
      }

      group.addEventListener('click', () => selectNode(node.id));
      group.addEventListener('keydown', (event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          selectNode(node.id);
        }
      });
      stage.append(group);
      dom.nodes.set(node.id, group);
    }

    // Every drawn edge carries its inferred payload class. Labels are painted
    // last (above nodes) and placed by deterministic candidate scoring:
    // midpoints at t=0.5/0.38/0.62/0.28/0.72 with perpendicular offsets
    // 0/±8/±14, then a nearby grid only when the path candidates are blocked.
    // A collision-free candidate keeps the payload half; otherwise the label
    // drops to the class token. On-path labels are placed first so they keep
    // the most legible spots.
    const labelContext = { nodes: data.nodes, hullLabels: data.hulls.map(hullLabelRect), stage: data.stage };
    const placed = [];
    const orderedForLabels = [...data.edges].sort(
      (a, b) => (Number(Boolean(b.onPath)) - Number(Boolean(a.onPath))) || (a.order - b.order),
    );
    for (const edge of orderedForLabels) {
      const unresolved = edge.label === 'unresolved' || edge.labelClass === 'unresolved';
      const label = edge.label || 'inferred';
      const chosen = placeEdgeLabel(edge, label, labelContext, placed);
      placed.push(inflateRect(chosen.box, 2));
      const el = svg('text', {
        class: `edge-label${edge.onPath ? ' is-onpath' : ''}${unresolved ? ' edge-label-unresolved' : ''}`,
        x: chosen.x,
        y: chosen.y,
        'text-anchor': chosen.anchor,
        'data-edge': `${edge.from}->${edge.to}`,
        'data-label': label,
        'data-source': chosen.source,
        'data-dropped': chosen.dropped ? 'payload' : 'none',
        ...fitted(chosen.text, LABEL_CHAR_W, LABEL_MAX_W),
      }, chosen.text);
      stage.append(el);
      dom.edgeLabels.push({
        el,
        data: edge,
        step: stepIndex.get(edge.from) ?? Number.POSITIVE_INFINITY,
      });
    }
  }

  /* ---------------------------------------------------------------- apply */

  function apply(ms) {
    const beats = state.beats;
    if (beats.length === 0) return;
    let index = 0;
    while (index < beats.length - 1 && ms >= beatStart(index + 1)) index += 1;
    state.index = index;
    state.ms = ms;

    const changed = index !== state.appliedIndex || state.selected !== state.appliedSelected;
    state.appliedIndex = index;
    state.appliedSelected = state.selected;

    const beat = beats[index];
    const focus = beat.focus || {};
    const full = ['boot', 'hottest', 'profile', 'start-here', 'stop', 'trust', 'summary'].includes(beat.kind);
    const step = beat.step || 0;

    if (!changed) {
      const scrubOnly = $('#scrub');
      if (scrubOnly) scrubOnly.value = String(Math.round(ms));
      return;
    }

    for (const node of state.data.nodes) {
      const group = dom.nodes.get(node.id);
      if (!group) continue;
      const active = node.id === focus.node;
      const visited = node.pathStep !== null && node.pathStep !== undefined && node.pathStep <= step;
      const dim = !full && !active && !visited && !node.hottest;
      group.classList.toggle('is-focus', active);
      group.classList.toggle('is-selected', state.selected === node.id);
      group.classList.toggle('is-visited', Boolean(node.pathStep) && visited);
      group.classList.toggle('is-dim', dim);
    }

    for (const entry of dom.edges) {
      const edge = entry.data;
      const incident = focus.node && (edge.from === focus.node || edge.to === focus.node);
      const reached = edge.onPath && entry.step <= step;
      const pendingPath = edge.onPath && !reached;
      entry.el.classList.toggle('is-current', edge.onPath && entry.step === step && beat.kind === 'enter');
      entry.el.classList.toggle('is-dim', !full && !edge.onPath && !incident);
      entry.el.style.opacity = pendingPath ? '0.22' : '';
    }

    for (const entry of dom.edgeLabels) {
      const edge = entry.data;
      const incident = focus.node && (edge.from === focus.node || edge.to === focus.node);
      const reached = edge.onPath && entry.step <= step;
      const pendingPath = edge.onPath && !reached;
      entry.el.classList.toggle('is-dim', !full && !edge.onPath && !incident);
      entry.el.style.opacity = pendingPath ? '0.22' : '';
    }

    const focusFunction = focus.fn
      ? ((state.byId.get(focus.node) || { functions: [] }).functions || []).find(
          (item) => item.id === focus.fn,
        )
      : null;
    for (const [id, meta] of dom.nodeTop) {
      const active = Boolean(focusFunction) && id === focus.node;
      const label = active
        ? `▶ ${midEllipsis(focusFunction.name, 18)} · L${focusFunction.line}`
        : meta.text;
      if (meta.el.textContent !== label) meta.el.textContent = label;
      meta.el.classList.toggle('is-fn', active);
    }

    setText('#narration-title', beat.title);
    setText('#narration-body', beat.narration);
    const chips = $('#narration-chips');
    chips.replaceChildren();
    for (const chip of beat.chips || []) {
      let variant = '';
      if (chip === 'inferred' || chip === 'synthetic') variant = ' inferred';
      else if (chip.startsWith('trip ')) variant = ' trip';
      chips.append(html('span', { class: `chip${variant}` }, chip));
    }

    setText('#beat-label', `beat ${index + 1}/${beats.length}`);
    const scrub = $('#scrub');
    if (scrub) scrub.value = String(Math.round(ms));
  }

  function setMs(ms) {
    const value = clamp(Math.round(ms), 0, Math.max(0, state.totalMs));
    apply(value);
  }

  /* --------------------------------------------------------------- player */

  function syncPlayButton() {
    const button = $('#play-pause');
    if (!button) return;
    button.replaceChildren();
    button.append(document.createTextNode(state.playing ? '❚❚ ' : '▶ '));
    button.append(html('span', {}, state.playing ? 'Pause' : 'Play'));
    button.setAttribute('aria-label', state.playing ? 'Pause story' : 'Play story');
  }

  function play() {
    if (state.playing || state.beats.length === 0) return;
    if (state.ms >= state.totalMs - 1) setMs(0);
    state.playing = true;
    lastTick = performance.now();
    syncPlayButton();
    frame = requestAnimationFrame(tick);
  }

  function pause() {
    if (!state.playing) return;
    state.playing = false;
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
    syncPlayButton();
  }

  function tick(now) {
    if (!state.playing) return;
    const delta = Math.min(120, now - lastTick);
    lastTick = now;
    setMs(state.ms + delta * state.speed);
    if (state.ms >= state.totalMs) {
      pause();
      return;
    }
    frame = requestAnimationFrame(tick);
  }

  function stepBy(delta) {
    pause();
    const target = clamp(state.index + delta, 0, state.beats.length - 1);
    setMs(beatStart(target));
  }

  function reset() {
    pause();
    closeInspector();
    setMs(0);
  }

  /* ------------------------------------------------------------ inspector */

  function hullFor(node) {
    const cx = node.x + node.w / 2;
    const cy = node.y + node.h / 2;
    const matches = state.data.hulls.filter(
      (hull) => cx >= hull.x && cx <= hull.x + hull.w && cy >= hull.y && cy <= hull.y + hull.h,
    );
    matches.sort((a, b) => a.w * a.h - b.w * b.h);
    return matches[0] || null;
  }

  function renderInspector(node) {
    const body = $('#inspector-body');
    body.replaceChildren();
    body.append(html('div', { class: 'insp-path' }, node.id));

    const meta = html('div', { class: 'insp-meta' });
    const addRow = (label, value) => {
      const row = html('div');
      row.append(html('b', {}, `${label}: `));
      row.append(document.createTextNode(value));
      meta.append(row);
    };
    addRow('class', `${node.entry ? 'entry · ' : ''}${node.nonproduction ? 'test/bench' : 'production'}${node.hottest ? ' · hottest' : ''}`);
    addRow('cost', `${node.costText} · ${node.ratioText}`);
    addRow('rank', node.topText);
    const hull = hullFor(node);
    if (hull) addRow('group', `${hull.label} (approximate prefix group)`);
    addRow('annotation', node.badge || 'none recorded');
    body.append(meta);

    const findings = [...state.data.findings]
      .filter((finding) => finding.path === node.id)
      .sort((a, b) => (SEVERITY[a.severity] ?? 9) - (SEVERITY[b.severity] ?? 9) || a.line - b.line);

    if (findings.length > 0) {
      const section = html('div', { class: 'insp-section' });
      section.append(html('h3', {}, `FINDINGS · ${findings.length}${findings.length > 8 ? ' (showing 8)' : ''}`));
      const list = html('ul', { class: 'insp-findings' });
      for (const finding of findings.slice(0, 8)) {
        const item = html('li');
        item.append(html('span', { class: `sev ${finding.severity}` }, finding.severity));
        item.append(document.createTextNode(` ${finding.ruleId} · line ${finding.line} · `));
        item.append(html('span', { class: 'rule' }, finding.title));
        list.append(item);
      }
      section.append(list);
      section.append(html('p', { class: 'insp-note' }, 'Static annotations; the map shows the dominant rule only.'));
      body.append(section);
    } else {
      body.append(html('p', { class: 'insp-empty' }, 'No findings recorded for this file in the data slice.'));
    }

    if (node.functions.length > 0) {
      const section = html('div', { class: 'insp-section' });
      section.append(html('h3', {}, 'TOP FUNCTIONS'));
      const table = html('table', { class: 'insp-table' });
      const head = html('tr');
      for (const label of ['function', 'line', 'cost', 'fan-out', 'loop', 'trip']) {
        head.append(html('th', {}, label));
      }
      table.append(head);
      const focusFn = state.beats[state.index] && state.beats[state.index].focus
        ? state.beats[state.index].focus.fn
        : null;
      for (const fn of node.functions) {
        const row = html('tr', fn.id === focusFn ? { class: 'is-fn' } : {});
        row.append(html('td', {}, fn.module ? `${fn.name} · module` : fn.name));
        row.append(html('td', {}, String(fn.line)));
        row.append(html('td', {}, fmt(fn.cost)));
        row.append(html('td', {}, String(fn.fanOut)));
        row.append(html('td', {}, String(fn.maxLoopDepth)));
        row.append(html('td', {}, fn.tripSource || 'unknown'));
        table.append(row);
      }
      section.append(table);
      body.append(section);
    }

    if (node.io) {
      const section = html('div', { class: 'insp-section' });
      const capped = node.io.rows.length < node.io.total
        ? ` · showing ${node.io.rows.length} of ${fmt(node.io.total)}`
        : '';
      section.append(html('h3', {}, `DATA STORES${capped}`));
      const list = html('ul', { class: 'insp-io' });
      for (const row of node.io.rows) {
        const item = html('li');
        item.append(html('span', { class: 'io-line' }, `L${row.line}`));
        item.append(html('span', { class: 'io-op' }, row.op));
        item.append(html('span', { class: 'io-target' }, row.target));
        if (row.literal !== null) {
          item.append(html('span', { class: 'io-literal' }, `"${row.literal}"`));
        } else if (row.target === 'stdio') {
          item.append(html('span', { class: 'io-unknown' }, 'stream · no path'));
        } else {
          item.append(html('span', { class: 'io-unknown' }, 'target unknown'));
        }
        list.append(item);
      }
      section.append(list);
      const unknownNote = node.io.unknownTarget
        ? ` ${fmt(node.io.unknownTarget)} argument target${node.io.unknownTarget === 1 ? '' : 's'} unknown (no plain literal).`
        : '';
      section.append(html('p', { class: 'insp-note' },
        `Static call-name match (v1 table); a literal is the written source target, not an observed path. Stdio rows carry no path.${unknownNote}`));
      body.append(section);
    }

    body.append(html('p', { class: 'insp-why' },
      `Why: ${node.ratioText} and ${node.topText} by static cost. Cost and trip counts are inferred, not measured; profile to confirm.`));
  }

  function selectNode(id) {
    state.selected = id;
    const node = state.byId.get(id);
    if (!node) return;
    renderInspector(node);
    $('#inspector').hidden = false;
    apply(state.ms);
  }

  function closeInspector() {
    state.selected = null;
    const inspector = $('#inspector');
    if (inspector) inspector.hidden = true;
    if (state.data) apply(state.ms);
  }

  /* ----------------------------------------------------------------- data */

  function showLoadError(error) {
    const panel = $('#load-error');
    const detail = $('#error-detail');
    if (detail) detail.textContent = String(error && error.message ? error.message : error);
    if (panel) panel.hidden = false;
  }

  async function loadCase(id) {
    pause();
    const entry = state.cases.find((candidate) => candidate.id === id) || state.cases[0];
    const response = await fetch(entry.file);
    if (!response.ok) throw new Error(`could not load ${entry.file} (${response.status})`);
    const data = await response.json();

    state.caseId = entry.id;
    state.data = data;
    state.synthetic = Boolean(data.synthetic);
    state.byId = new Map(data.nodes.map((node) => [node.id, node]));
    state.beats = buildBeats(data);
    state.totalMs = state.beats.reduce((total, beat) => total + (DURATION[beat.kind] || 2000), 0);
    state.selected = null;
    state.appliedIndex = -1;
    state.appliedSelected = null;
    $('#inspector').hidden = true;

    renderMeta();
    renderGraph();

    const stub = data.meta.stub;
    const stubPanel = $('#stub');
    stubPanel.hidden = !stub;
    if (stub) {
      setText('#stub-title', stub.title);
      setText('#stub-body', stub.body);
    }

    const scrub = $('#scrub');
    scrub.max = String(state.totalMs);
    scrub.value = '0';

    const start = state.reduced ? state.beats.length - 1 : 0;
    setMs(beatStart(start));
    syncPlayButton();
  }

  function fillCases(cases) {
    const select = $('#case-select');
    select.replaceChildren();
    for (const entry of cases) {
      select.append(html('option', { value: entry.id }, entry.label));
    }
  }

  /* ---------------------------------------------------------------- wiring */

  function wire() {
    $('#play-pause').addEventListener('click', () => (state.playing ? pause() : play()));
    $('#step-back').addEventListener('click', () => stepBy(-1));
    $('#step-forward').addEventListener('click', () => stepBy(1));
    $('#reset').addEventListener('click', reset);
    $('#scrub').addEventListener('input', (event) => {
      pause();
      setMs(Number(event.target.value));
    });
    $('#speed').addEventListener('change', (event) => {
      state.speed = Number(event.target.value) || 1;
    });
    $('#case-select').addEventListener('change', (event) => {
      loadCase(event.target.value).catch(showLoadError);
    });
    $('#inspector-close').addEventListener('click', closeInspector);

    document.addEventListener('keydown', (event) => {
      const tag = (event.target.tagName || '').toLowerCase();
      if (tag === 'input' || tag === 'select' || tag === 'textarea') return;
      if (event.key === ' ') {
        event.preventDefault();
        if (state.playing) pause(); else play();
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        stepBy(1);
      } else if (event.key === 'ArrowLeft') {
        event.preventDefault();
        stepBy(-1);
      } else if (event.key === 'Home') {
        event.preventDefault();
        reset();
      } else if (event.key === 'Escape') {
        closeInspector();
      } else if (SPEEDS[event.key]) {
        state.speed = SPEEDS[event.key];
        const select = $('#speed');
        select.value = String(state.speed);
      }
    });

    window.addEventListener('beforeprint', () => {
      pause();
      setMs(beatStart(state.beats.length - 1));
    });
  }

  async function boot() {
    state.reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    wire();
    try {
      const response = await fetch('data-index.json');
      if (!response.ok) throw new Error(`could not load data-index.json (${response.status})`);
      const index = await response.json();
      state.cases = index.cases;
      fillCases(state.cases);
      await loadCase(state.cases[0].id);
    } catch (error) {
      showLoadError(error);
    }
  }

  window.__sim = {
    state: () => ({
      caseId: state.caseId,
      index: state.index,
      beats: state.beats.length,
      playing: state.playing,
      speed: state.speed,
      totalMs: state.totalMs,
      ms: state.ms,
      selected: state.selected,
      synthetic: state.synthetic,
      kinds: state.beats.map((beat) => beat.kind),
      kind: state.beats[state.index] ? state.beats[state.index].kind : null,
      focus: state.beats[state.index] ? state.beats[state.index].focus || {} : {},
    }),
    seekIndex(index) {
      pause();
      setMs(beatStart(clamp(index, 0, state.beats.length - 1)));
    },
    setCase(id) {
      return loadCase(id);
    },
    applyFinal() {
      pause();
      setMs(beatStart(state.beats.length - 1));
    },
  };

  boot();
})();
