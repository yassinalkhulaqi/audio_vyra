function boot() {
    const invoke = window.__TAURI__.core.invoke;
    let toastTimer = null;
    function toast(message, kind) {
      const el = document.getElementById('toast');
      if (!el) return;
      el.textContent = String(message);
      el.className = 'toast show ' + (kind || 'error');
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => { el.className = 'toast'; }, 3200);
    }
    async function refreshDevices() {
      const ins = await invoke('list_input_devices');
      const outs = await invoke('list_output_devices');
      const isel = document.getElementById('input');
      const osel = document.getElementById('output');
      const fill = (sel, devices) => {
        sel.replaceChildren(...devices.map(d => {
          const o = document.createElement('option');
          o.value = d; o.textContent = d;
          return o;
        }));
      };
      fill(isel, ins);
      fill(osel, outs);
    };
    document.getElementById('start').onclick = async () => {
      const input = document.getElementById('input').value || null;
      const output = document.getElementById('output').value || null;
      try {
        await invoke('start_engine', { input, output });
        document.getElementById('status').textContent = 'Running';
        const si = await invoke('stream_info');
        document.getElementById('stream_info').textContent = `in ${si[0]} Hz · out ${si[1]} Hz · ${si[2]} ch` + (si[0] !== si[1] ? ' (resampling)' : '');
      } catch(e) { toast('Start failed: ' + e); }
    };
    document.getElementById('stop').onclick = async () => {
      await invoke('stop_engine');
      document.getElementById('status').textContent = 'Stopped';
    };
    document.getElementById('gain').oninput = async (e) => {
      const v = parseFloat(e.target.value);
      document.getElementById('gainval').textContent = v;
      await invoke('set_input_gain', { db: v });
    };
    setInterval(async () => {
      try {
        const l = await invoke('levels');
        const rms = l.rms || 0;
        document.getElementById('bar').style.width = Math.min(100, rms*200) + '%';
        document.getElementById('lvl').textContent = rms.toFixed(3);
        document.getElementById('peak').textContent = (l.peak || 0).toFixed(3);
        const latency = await invoke('buffered_ms');
        document.getElementById('latency').textContent = latency + ' ms buffered';
        rmsHistory.push(rms); rmsHistory.shift(); drawRms();
        document.getElementById('bar').style.background = l.clipped ? '#d29922' : '#2ea043';
        document.getElementById('clip').style.color = l.clipped ? '#f85149' : '#8b949e';
        const st = await invoke('audio_stats');
        document.getElementById('audio_stats').textContent = `overruns ${st[0]} · underruns ${st[1]}`;
        const rt = await invoke('rt_stats');
        document.getElementById('rt_stats').textContent = `skips fx ${rt[0]} · fft ${rt[1]} · rec ${rt[2]} · rate ${rt[3]}`;
      } catch(e) {}
    }, 100);
    let dl = [300, 0.35, 0.3, 0.2];
    function pushDelay() { invoke('set_delay', { timeMs: dl[0], feedback: dl[1], mix: dl[2], damp: dl[3] }); }
    [['time',0],['fb',1],['mix',2],['damp',3]].forEach(([key,i]) => {
      const el = document.getElementById('dl_' + key);
      el.oninput = (e) => {
        dl[i] = parseFloat(e.target.value);
        document.getElementById('dl_' + key + 'v').textContent = key === 'time' ? dl[i] + ' ms' : dl[i].toFixed(2);
        pushDelay();
      };
    });
    (async () => {
      dl = await invoke('get_delay');
      [['time',0],['fb',1],['mix',2],['damp',3]].forEach(([key,i]) => {
        const el = document.getElementById('dl_' + key);
        el.value = dl[i];
        document.getElementById('dl_' + key + 'v').textContent = key === 'time' ? dl[i] + ' ms' : dl[i].toFixed(2);
      });
    })();
    let rv = [0.6, 0.4, 0.25, 0.85];
    function pushReverb() { invoke('set_reverb', { roomSize: rv[0], damp: rv[1], wet: rv[2], dry: rv[3] }); }
    [['room',0],['damp',1],['wet',2],['dry',3]].forEach(([key,i]) => {
      const el = document.getElementById('rv_' + key);
      el.oninput = (e) => {
        rv[i] = parseFloat(e.target.value);
        document.getElementById('rv_' + key + 'v').textContent = rv[i].toFixed(2);
        pushReverb();
      };
    });
    (async () => {
      rv = await invoke('get_reverb');
      [['room',0],['damp',1],['wet',2],['dry',3]].forEach(([key,i]) => {
        const el = document.getElementById('rv_' + key);
        el.value = rv[i];
        document.getElementById('rv_' + key + 'v').textContent = rv[i].toFixed(2);
      });
    })();
    let ds = [0, 12, 0.6, 0.5, 1, 8];
    function pushDist() { invoke('set_distortion', { mode: ds[0], driveDb: ds[1], tone: ds[2], mix: ds[3], downsample: ds[4], bitDepth: ds[5] }); }
    const dsMode = document.getElementById('ds_mode');
    dsMode.onchange = (e) => { ds[0] = parseInt(e.target.value); pushDist(); };
    const distFmt = (key, v) => key === 'drive' ? v + ' dB' : (key === 'down' ? v + 'x' : (key === 'bits' ? String(v) : v.toFixed(2)));
    [['drive',1],['tone',2],['mix',3],['down',4],['bits',5]].forEach(([key,i]) => {
      const el = document.getElementById('ds_' + key);
      el.oninput = (e) => {
        ds[i] = parseFloat(e.target.value);
        document.getElementById('ds_' + key + 'v').textContent = distFmt(key, ds[i]);
        pushDist();
      };
    });
    (async () => {
      ds = await invoke('get_distortion');
      dsMode.value = ds[0];
      [['drive',1],['tone',2],['mix',3],['down',4],['bits',5]].forEach(([key,i]) => {
        const el = document.getElementById('ds_' + key);
        el.value = ds[i];
        document.getElementById('ds_' + key + 'v').textContent = distFmt(key, ds[i]);
      });
    })();
    let mo = [0, 1.5, 0.4, 0.3, 0.5];
    function pushMod() { invoke('set_modulation', { mode: mo[0], rateHz: mo[1], depth: mo[2], feedback: mo[3], mix: mo[4] }); }
    const moMode = document.getElementById('mo_mode');
    moMode.onchange = (e) => { mo[0] = parseInt(e.target.value); pushMod(); };
    [['rate',1,' Hz'],['depth',2,''],['fb',3,''],['mix',4,'']].forEach(([key,i,suffix]) => {
      const el = document.getElementById('mo_' + key);
      el.oninput = (e) => {
        mo[i] = parseFloat(e.target.value);
        document.getElementById('mo_' + key + 'v').textContent = (suffix === ' Hz' ? mo[i].toFixed(1) + ' Hz' : mo[i].toFixed(2));
        pushMod();
      };
    });
    (async () => {
      mo = await invoke('get_modulation');
      moMode.value = mo[0];
      [['rate',1,' Hz'],['depth',2,''],['fb',3,''],['mix',4,'']].forEach(([key,i,suffix]) => {
        const el = document.getElementById('mo_' + key);
        el.value = mo[i];
        document.getElementById('mo_' + key + 'v').textContent = (suffix === ' Hz' ? mo[i].toFixed(1) + ' Hz' : mo[i].toFixed(2));
      });
    })();
    async function loadEq() {
      const bands = await invoke('get_eq_bands');
      const box = document.getElementById('eq');
      box.innerHTML = '';
      const kinds = ['Peaking','Low Shelf','High Shelf','Low Pass','High Pass'];
      bands.forEach((b, i) => {
        const state = Object.assign({}, b);
        const row = document.createElement('div');
        row.className = 'row';
        row.style.flexWrap = 'wrap';

        const title = document.createElement('span');
        title.style.width = '70px';
        title.textContent = 'Band ' + (i + 1);

        const kindSel = document.createElement('select');
        kinds.forEach((k, ki) => {
          const o = document.createElement('option');
          o.value = String(ki);
          o.textContent = k;
          if (ki === b.kind) o.selected = true;
          kindSel.appendChild(o);
        });
        kindSel.onchange = () => { state.kind = parseInt(kindSel.value); pushBand(); };

        function slider(min, max, step, key, fmt) {
          const wrap = document.createElement('span');
          wrap.style.display = 'inline-flex';
          wrap.style.alignItems = 'center';
          wrap.style.gap = '4px';
          const input = document.createElement('input');
          input.type = 'range';
          input.min = min; input.max = max; input.step = step;
          input.value = state[key];
          const span = document.createElement('span');
          span.textContent = fmt(state[key]);
          input.oninput = () => {
            state[key] = parseFloat(input.value);
            span.textContent = fmt(state[key]);
            pushBand();
          };
          wrap.append(input, span);
          return wrap;
        }

        const freqEl = slider(20, 18000, 1, 'frequency', v => v.toFixed(0) + ' Hz');
        const qEl = slider(0.1, 18, 0.1, 'q', v => v.toFixed(2));
        const gainEl = slider(-18, 18, 0.5, 'gain_db', v => v.toFixed(1) + ' dB');

        const en = document.createElement('input');
        en.type = 'checkbox';
        en.checked = state.enabled;
        en.title = 'Enabled';
        en.onchange = () => { state.enabled = en.checked; pushBand(); };

        const del = document.createElement('button');
        del.textContent = '\u00d7';
        del.title = 'Remove band';
        del.onclick = async () => {
          await invoke('remove_eq_band', { index: i });
          await loadEq();
        };

        function pushBand() {
          invoke('set_eq_band', {
            index: i, kind: state.kind, frequency: state.frequency,
            q: state.q, gainDb: state.gain_db, enabled: state.enabled
          });
        }

        row.append(title, kindSel, freqEl, qEl, gainEl, en, del);
        box.appendChild(row);
      });

      const addRow = document.createElement('div');
      addRow.className = 'row';
      const addBtn = document.createElement('button');
      addBtn.textContent = '+ Add band';
      addBtn.onclick = async () => {
        await invoke('add_eq_band', { kind: 0, frequency: 1000, q: 1.0, gainDb: 0 });
        await loadEq();
      };
      addRow.appendChild(addBtn);
      box.appendChild(addRow);
    }

    const pitch = document.getElementById('pitch');
    pitch.oninput = async (e) => {
      const v = parseFloat(e.target.value);
      document.getElementById('pitchval').textContent = v + ' st';
      await invoke('set_pitch', { semitones: v });
    };
    (async () => {
      const v = await invoke('get_pitch');
      pitch.value = v;
      document.getElementById('pitchval').textContent = v + ' st';
    })();
    const outGain = document.getElementById('out_gain');
    outGain.oninput = async (e) => {
      const v = parseFloat(e.target.value);
      document.getElementById('out_gainv').textContent = v + ' dB';
      await invoke('set_output_gain', { db: v });
    };
    document.getElementById('out_mute').onchange = async (e) => {
      await invoke('set_output_muted', { muted: e.target.checked });
    };
    (async () => {
      const db = await invoke('get_output_gain');
      outGain.value = db;
      document.getElementById('out_gainv').textContent = db.toFixed(1) + ' dB';
      document.getElementById('out_mute').checked = await invoke('is_output_muted');
    })();
    const canvas = document.getElementById('spectrum');
    const ctx = canvas.getContext('2d');
    let peakHold = [];
    async function drawSpectrum() {
      try {
        const bins = await invoke('spectrum');
        ctx.clearRect(0,0,canvas.width,canvas.height);
        if (bins.length) {
          const logMax = Math.log(bins.length + 1);
          while (peakHold.length < bins.length) peakHold.push(-120);
          for (let i=0;i<bins.length;i++) {
            const db = bins[i];
            const x = Math.log(i + 1) / logMax * canvas.width;
            const xNext = Math.log(i + 2) / logMax * canvas.width;
            const h = Math.max(0, Math.min(1, (db + 90) / 90)) * canvas.height;
            ctx.fillStyle = '#2ea043';
            ctx.fillRect(x, canvas.height-h, Math.max(1, xNext-x), h);
            if (db > peakHold[i]) peakHold[i] = db;
            else peakHold[i] = Math.max(db, peakHold[i] - 1.5);
            const ph = Math.max(0, Math.min(1, (peakHold[i] + 90) / 90)) * canvas.height;
            ctx.fillStyle = '#f85149';
            ctx.fillRect(x, canvas.height-ph, Math.max(1, xNext-x), 2);
          }
        }
      } catch(e) {}
      requestAnimationFrame(drawSpectrum);
    }
    drawSpectrum();
    const rmsCanvas = document.getElementById('rms_chart');
    const rmsCtx = rmsCanvas.getContext('2d');
    const rmsHistory = new Array(120).fill(0);
    function drawRms() {
      rmsCtx.clearRect(0,0,rmsCanvas.width,rmsCanvas.height);
      rmsCtx.strokeStyle = '#2ea043';
      rmsCtx.lineWidth = 2;
      rmsCtx.beginPath();
      rmsHistory.forEach((v,i) => {
        const x = i / (rmsHistory.length - 1) * rmsCanvas.width;
        const y = rmsCanvas.height - Math.min(1, v * 3) * rmsCanvas.height;
        if (i === 0) rmsCtx.moveTo(x, y); else rmsCtx.lineTo(x, y);
      });
      rmsCtx.stroke();
    }
    document.getElementById('rec_start').onclick = async () => { await invoke('start_recording'); };
    document.getElementById('rec_stop').onclick = async () => { await invoke('stop_recording'); };
    document.getElementById('rec_save').onclick = async () => {
      try { await invoke('save_recording', { path: document.getElementById('rec_path').value }); }
      catch(e) { toast('Save failed: ' + e); }
    };
    document.getElementById('play_btn').onclick = async () => {
      try { await invoke('play_wav', { path: document.getElementById('play_path').value }); }
      catch(e) { toast('Playback failed: ' + e); }
    };
    document.getElementById('play_stop').onclick = async () => { await invoke('stop_playback'); };
    setInterval(async () => {
      try {
        const rec = await invoke('is_recording');
        const n = await invoke('recorded_samples');
        document.getElementById('rec_state').textContent = rec ? 'Recording' : 'Idle';
        document.getElementById('rec_count').textContent = n + ' samples';
      } catch(e) {}
    }, 250);
    const prDir = () => document.getElementById('pr_dir').value;
    const num = (id) => parseFloat(document.getElementById(id).value) || 0;
    function collectPresetParams() {
      return {
        gain_db: num('gain'), output_gain_db: num('out_gain'),
        output_muted: document.getElementById('out_mute').checked ? 1 : 0,
        delay_time: num('dl_time'), delay_feedback: num('dl_fb'), delay_mix: num('dl_mix'), delay_damp: num('dl_damp'),
        reverb_room: num('rv_room'), reverb_damp: num('rv_damp'), reverb_wet: num('rv_wet'), reverb_dry: num('rv_dry'),
        dist_mode: num('ds_mode'), dist_drive: num('ds_drive'), dist_tone: num('ds_tone'),
        dist_mix: num('ds_mix'), dist_down: num('ds_down'), dist_bits: num('ds_bits'),
        mod_mode: num('mo_mode'), mod_rate: num('mo_rate'), mod_depth: num('mo_depth'), mod_fb: num('mo_fb'), mod_mix: num('mo_mix'),
        pitch: num('pitch'),
        vocal_mode: vo[0], vocal_carrier: vo[1], vowel: vo[2], vocal_depth: vo[3], vocal_mix: vo[4],
      };
    }
    async function refreshPresets() {
      try {
        const names = await invoke('list_presets', { dir: prDir() });
        const sel = document.getElementById('pr_list');
        sel.replaceChildren(...names.map(n => {
          const o = document.createElement('option');
          o.value = n; o.textContent = n;
          return o;
        }));
      } catch(e) {}
    }
    document.getElementById('pr_save').onclick = async () => {
      const s = await invoke('get_state');
      const preset = {
        name: document.getElementById('pr_name').value,
        params: collectPresetParams(),
        enabled: s.enabled,
        chain: s.chain,
        eq_bands: s.eq_bands,
      };
      try { await invoke('save_preset', { dir: prDir(), preset }); refreshPresets(); toast('Preset saved', 'ok'); }
      catch(e) { toast('Save failed: ' + e); }
    };
    document.getElementById('pr_load').onclick = async () => {
      const name = document.getElementById('pr_list').value;
      if (!name) return;
      try {
        const p = await invoke('load_preset', { dir: prDir(), name });
        await invoke('apply_preset', { preset: p });
        await syncAll();
        document.getElementById('pr_info').textContent = 'Loaded ' + p.name;
      } catch(e) { toast('Load failed: ' + e); }
    };
    document.getElementById('pr_delete').onclick = async () => {
      const name = document.getElementById('pr_list').value;
      if (!name) return;
      try { await invoke('delete_preset', { dir: prDir(), name }); refreshPresets(); }
      catch(e) { toast('Delete failed: ' + e); }
    };
    document.getElementById('pr_factory').onclick = async () => {
      try { await invoke('install_factory_presets', { dir: prDir() }); refreshPresets(); }
      catch(e) { toast('Install failed: ' + e); }
    };
    refreshPresets();
    async function refreshToggles() {
      try {
        const names = await invoke('list_effects');
        const box = document.getElementById('fx_toggles');
        box.replaceChildren();
        for (const n of names) {
          const label = document.createElement('label');
          label.style.marginRight = '14px';
          const cb = document.createElement('input');
          cb.type = 'checkbox';
          cb.checked = await invoke('effect_enabled', { name: n });
          cb.onchange = async () => {
            await invoke('set_effect_enabled', { name: n, enabled: cb.checked });
            await refreshToggles();
            await loadChain();
          };
          label.append(cb, document.createTextNode(' ' + n));
          box.appendChild(label);
        }
      } catch(e) {}
    }
    refreshToggles();
    async function loadChain() {
      try {
        const names = await invoke('list_effects');
        const box = document.getElementById('fx_chain');
        box.replaceChildren();
        for (const n of names) {
          const row = document.createElement('div');
          row.className = 'row';
          row.draggable = true;
          row.dataset.effect = n;
          row.ondragstart = (e) => {
            e.dataTransfer.setData('text/plain', n);
            e.dataTransfer.effectAllowed = 'move';
            row.classList.add('dragging');
          };
          row.ondragend = () => row.classList.remove('dragging');
          row.ondragover = (e) => { e.preventDefault(); e.dataTransfer.dropEffect = 'move'; };
          row.ondrop = async (e) => {
            e.preventDefault();
            const source = e.dataTransfer.getData('text/plain');
            if (!source || source === n) return;
            const rows = [...box.querySelectorAll('[data-effect]')];
            const order = rows.map((r) => r.dataset.effect);
            const from = order.indexOf(source);
            const to = order.indexOf(n);
            if (from < 0 || to < 0) return;
            order.splice(to, 0, order.splice(from, 1)[0]);
            await invoke('reorder_effects', { names: order });
            await loadChain();
          };
          const cb = document.createElement('input');
          cb.type = 'checkbox';
          cb.checked = await invoke('effect_enabled', { name: n });
          cb.onchange = async () => {
            await invoke('set_effect_enabled', { name: n, enabled: cb.checked });
            await refreshToggles();
          };
          const label = document.createElement('span');
          label.textContent = n;
          const up = document.createElement('button');
          up.textContent = '\u2191';
          up.onclick = async () => { await invoke('move_effect', { name: n, delta: -1 }); await loadChain(); };
          const down = document.createElement('button');
          down.textContent = '\u2193';
          down.onclick = async () => { await invoke('move_effect', { name: n, delta: 1 }); await loadChain(); };
          row.append(cb, label, up, down);
          box.appendChild(row);
        }
      } catch(e) {}
    }
    loadChain();
    function setVal(id, v, dispId, text) {
      const el = document.getElementById(id);
      if (el) el.value = v;
      const disp = document.getElementById(dispId);
      if (disp) disp.textContent = text;
    }
    async function syncAll() {
      const s = await invoke('get_state');
      setVal('gain', s.input_gain_db, 'gainval', String(s.input_gain_db));
      setVal('out_gain', s.output_gain_db, 'out_gainv', s.output_gain_db.toFixed(1) + ' dB');
      document.getElementById('out_mute').checked = s.output_muted;
      dl = s.delay.slice(); rv = s.reverb.slice(); ds = s.distortion.slice(); mo = s.modulation.slice(); vo = s.vocal.slice();
      [['time',0],['fb',1],['mix',2],['damp',3]].forEach(([k,i]) => {
        setVal('dl_' + k, dl[i], 'dl_' + k + 'v', k === 'time' ? dl[i] + ' ms' : dl[i].toFixed(2));
      });
      [['room',0],['damp',1],['wet',2],['dry',3]].forEach(([k,i]) => {
        setVal('rv_' + k, rv[i], 'rv_' + k + 'v', rv[i].toFixed(2));
      });
      [['drive',1],['tone',2],['mix',3],['down',4],['bits',5]].forEach(([k,i]) => {
        setVal('ds_' + k, ds[i], 'ds_' + k + 'v', distFmt(k, ds[i]));
      });
      dsMode.value = ds[0];
      [['rate',1],['depth',2],['fb',3],['mix',4]].forEach(([k,i]) => {
        setVal('mo_' + k, mo[i], 'mo_' + k + 'v', k === 'rate' ? mo[i].toFixed(1) + ' Hz' : mo[i].toFixed(2));
      });
      moMode.value = mo[0];
      setVal('pitch', s.pitch_semitones, 'pitchval', s.pitch_semitones + ' st');
      syncVocalUI();
      await loadEq();
      await refreshToggles();
      await loadChain();
    }
    document.getElementById('st_dump').onclick = async () => {
      try { document.getElementById('st_json').value = JSON.stringify(await invoke('get_state'), null, 2); }
      catch(e) { toast('Dump failed: ' + e); }
    };
    document.getElementById('st_apply').onclick = async () => {
      try {
        await invoke('set_state', { newState: JSON.parse(document.getElementById('st_json').value) });
        await syncAll();
      } catch(e) { toast('Apply failed: ' + e); }
    };
    document.getElementById('cfg_save').onclick = async () => {
      const cfg = {
        input_device: document.getElementById('input').value || null,
        output_device: document.getElementById('output').value || null,
        params: { gain_db: parseFloat(document.getElementById('gain').value) || 0 },
        state: await invoke('get_state'),
      };
      try { await invoke('save_config', { cfg }); toast('Configuration saved', 'ok'); }
      catch(e) { toast('Save config failed: ' + e); }
    };
    document.getElementById('cfg_load').onclick = async () => {
      try {
        const cfg = await invoke('load_config');
        if (cfg.input_device) document.getElementById('input').value = cfg.input_device;
        if (cfg.output_device) document.getElementById('output').value = cfg.output_device;
        if (cfg.state) { await invoke('set_state', { newState: cfg.state }); await syncAll(); }
        else if (typeof cfg.params.gain_db === 'number') {
          await invoke('set_input_gain', { db: cfg.params.gain_db });
          await syncAll();
        }
      } catch(e) { toast('Load config failed: ' + e); }
    };
    document.getElementById('stats_reset').onclick = async () => { await invoke('reset_audio_stats'); };
    document.getElementById('rt_stats_reset').onclick = async () => { await invoke('reset_rt_stats'); };
    document.getElementById('clip_reset').onclick = async () => { await invoke('reset_clip'); };
    let vo = [0, 40, 0, 0.6, 0.6];
    const vowels = ['A','E','I','O','U'];
    function pushVocal() { invoke('set_vocal', { mode: vo[0], carrierHz: vo[1], vowel: vo[2], depth: vo[3], mix: vo[4] }); }
    const voMode = document.getElementById('vo_mode');
    voMode.onchange = (e) => { vo[0] = parseInt(e.target.value); pushVocal(); };
    const vowelFmt = (v) => vowels[Math.round(v)] || 'A';
    function syncVocalUI() {
      voMode.value = vo[0];
      [['car',1,' Hz'],['vowel',2,'v'],['depth',3,''],['mix',4,'']].forEach(([key,i,suffix]) => {
        const el = document.getElementById('vo_' + key);
        el.value = vo[i];
        document.getElementById('vo_' + key + 'v').textContent = suffix === 'v' ? vowelFmt(vo[i]) : (suffix === ' Hz' ? vo[i] + ' Hz' : vo[i].toFixed(2));
      });
    }
    [['car',1,' Hz'],['vowel',2,'v'],['depth',3,''],['mix',4,'']].forEach(([key,i,suffix]) => {
      const el = document.getElementById('vo_' + key);
      el.oninput = (e) => {
        vo[i] = parseFloat(e.target.value);
        document.getElementById('vo_' + key + 'v').textContent = suffix === 'v' ? vowelFmt(vo[i]) : (suffix === ' Hz' ? vo[i] + ' Hz' : vo[i].toFixed(2));
        pushVocal();
      };
    });
    (async () => {
      vo = await invoke('get_vocal');
      voMode.value = vo[0];
      [['car',1,' Hz'],['vowel',2,'v'],['depth',3,''],['mix',4,'']].forEach(([key,i,suffix]) => {
        const el = document.getElementById('vo_' + key);
        el.value = vo[i];
        document.getElementById('vo_' + key + 'v').textContent = suffix === 'v' ? vowelFmt(vo[i]) : (suffix === ' Hz' ? vo[i] + ' Hz' : vo[i].toFixed(2));
      });
    })();
    document.querySelectorAll('label').forEach((label) => {
      const next = label.nextElementSibling;
      if (next && !label.htmlFor && ['INPUT', 'SELECT', 'TEXTAREA'].includes(next.tagName)) {
        label.htmlFor = next.id;
      }
    });
    document.addEventListener('keydown', async (e) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const target = e.target;
      if (target && ['INPUT', 'SELECT', 'TEXTAREA'].includes(target.tagName)) return;
      if (e.code === 'Space') {
        e.preventDefault();
        document.getElementById(e.shiftKey ? 'stop' : 'start').click();
      } else if (e.key.toLowerCase() === 'r') {
        const recording = await invoke('is_recording');
        document.getElementById(recording ? 'rec_stop' : 'rec_start').click();
      } else if (e.key.toLowerCase() === 'm') {
        const box = document.getElementById('out_mute');
        box.checked = !box.checked;
        await invoke('set_output_muted', { muted: box.checked });
        toast('Output ' + (box.checked ? 'muted' : 'unmuted'), 'ok');
      }
    });
    refreshDevices();
    loadEq();
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', boot);
  else boot();

