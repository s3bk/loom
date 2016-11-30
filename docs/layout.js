let layout_items;

function layout(items) {
    layout_items = items;
}

let config = {
    space_shrink:   0.5,
    space_width:    1.0,
    space_stretch:  2.0,
    text_width:     50
};

let display_state = {
    cache: {},
    time:   0
};

document.addEventListener("DOMContentLoaded", function() {
    let p = document.getElementById("controls");
    
    add_control(p, "space_shrink", 0.2, 1.0, 0.05);
    add_control(p, "space_width", 0.6, 2.0, 0.2);
    add_control(p, "space_stretch", 1.0, 5.0, 0.25);
    add_control(p, "text_width", 0, 100, 1, function(v) {
        document.getElementById("target").style.width = v + "vw";
    });
    
    update_layout();
}, true);

function add_control(dl, name, min, max, step, callback) {
    let dt = document.createElement("dt");
    dt.appendChild(document.createTextNode(name));
    dl.appendChild(dt);
    
    let dd = document.createElement("dd");
    let i = document.createElement("input");
    i.setAttribute("type", "range");
    i.setAttribute("min",  min);
    i.setAttribute("max", max);
    i.setAttribute("step", step);
    
    let value = localStorage[name];
    if (value != undefined) {
        i.setAttribute("value", value);
        config[name] = value;
        if (callback != undefined) callback(value);
    }
    
    i.addEventListener("input", function(e) {
        let v = e.target.value;
        config[name] = v;
        localStorage[name] = v;
        e.target.setAttribute("value", v);
        console.log(name, "=", v);
        if (callback != undefined) callback(v);
        update_layout();
    }, false);
    dd.appendChild(i);
    dl.appendChild(dd);
}
function update_layout() {
    let items = layout_items;
    
    let parent = document.getElementById("target");
    while (parent.firstChild) {
        parent.removeChild(parent.firstChild);
    }
    let cache = display_state.cache;
    if (cache.space_width == undefined) {
        let s = document.createElement("span");
        s.innerHTML = "&nbsp;";
        parent.appendChild(s);
        cache.space_width = s.getBoundingClientRect().width;
        parent.removeChild(s);
    }
    
    let context = {
        word:   function(word)
                {
                    let measure = cache[word];
                    if (measure == undefined) {
                        let span = document.createElement("span");
                        span.appendChild(document.createTextNode(word));
                        parent.appendChild(span);
                        let rect = span.getBoundingClientRect();
                        parent.removeChild(span);
                        
                        measure = {
                            shrink:     rect.width,
                            width:      rect.width,
                            stretch:    rect.width,
                            height:     rect.height
                        };
                        cache[word] = measure;
                    }
                    return measure
                },
        space:  function(scale)
                {
                    let width = cache.space_width * scale;
                    return {
                        shrink:     width * config.space_width * config.space_shrink,
                        width:      width * config.space_width,
                        stretch:    width * config.space_width * config.space_stretch,
                        height:     0.
                    };
                },
        width:  parent.getBoundingClientRect().width,
        items:  items
    };
    
    let lines = run(context);
    display_state.lines = lines;
    display_state.line = 0;
    display_state.y = 0.;
    display_state.target = parent;
    display_state.time = new Date();
    
    window.setTimeout(show_line, 1);
}

function max(a, b) {
    return a > b ? a : b;
}

function measure_at(measure, factor) {
    var d;
    if (factor < 0.) {
        d = measure.width - measure.shrink;
    } else {
        d = measure.stretch - measure.width;
    }
    
    return d * factor + measure.width;
}

function measure_add(a, b) {
    return {
        shrink:     a.shrink  + b.shrink,
        width:      a.width   + b.width,
        stretch:    a.stretch + b.stretch,
        height: max(a.height,   b.height)
    };
}

function run(self) {
    let limit = self.items.length;
    let nodes = [{
        prev:   0,
        path:   0,
        factor: 0.,
        score:  0.
    }];
    let last = 0;
        
    for (var start = 0; start < limit; start++) {  
        let b = nodes[start];
        if (b != undefined) {
            last = complete_line(self, nodes, {
                measure: {
                    shrink:     0.,
                    width:      0.,
                    stretch:    0.,
                    height:     0.
                },
                path:       0,
                score:      b.score,
                begin:      start,
                pos:        start,
                branches:   0
            });
        }
    }
    
    let steps = [];
        
    while (last > 0) {
        let b = nodes[last];
        if (b == undefined) {
            console.error();
        }
        steps.push([b, last-1]);
        last = b.prev;
    }
    let lines = [];
    for (var step of steps.reverse()) {
        let b = step[0];
        let end = step[1];
        
        let measure = {
            shrink:     0.,
            width:      0.,
            stretch:    0.,
            height:     0.
        };
        let words = [];
        let pos = b.prev;
        let branches = 0;
        while (pos < end) {
            let node = self.items[pos];
            switch (node[0]) {
                case 0: // Word
                    let w = node[1];
                    let x = measure_at(measure, b.factor);
                    measure = measure_add(measure, self.word(w));
                    words.push([w, x]);
                    break;
                
                case 2: // Space
                    let s = node[2];
                    measure = measure_add(measure, self.space(s));
                    break;
                
                case 3: // BranchEntry
                    let len = node[1];
                    if ((b.path & (1<<branches)) == 0) {
                        pos += len;
                    }
                    branches += 1;
                    break;
                
                case 4: // BranchExit
                    let skip = node[1];
                    pos += skip;
                    break;
            }
            pos += 1;
        }
        
        lines.push([measure.height, words]);
    }
    return lines;
}
    
function complete_line(self, nodes, c) {
    let last = c.begin;
    
    while (c.pos < self.items.length) {
        let n = c.pos;
        let item = self.items[n];
        switch (item[0]) {
            case 0: // Word
                let w = item[1];
                c.measure = measure_add(c.measure, self.word(w));
                break;
            
            case 2: // Space
                let breaking = item[1];
                let s = item[2];
                if (breaking) {
                    if (maybe_update(self, c, nodes, n+1)) {
                        last = n+1;
                    }
                }
                
                // add width now.
                c.measure = measure_add(c.measure, self.space(s));
                break;
            
            case 1: // Linebreak
                let fill = item[1];
                if (fill) {
                    if (self.width > c.measure.stretch) {
                        c.measure.stretch = self.width;
                        if (self.width > c.measure.width) {
                            c.measure.width = self.width;
                        }
                    }
                }
            
                if (maybe_update(self, c, nodes, n+1)) {
                    last = n+1;
                }
                return last;
            
            case 3: // BranchEntry
                let len = item[1];
                let b_last = complete_line(self, nodes, {
                    pos:        n + 1,
                    path:       c.path | (1 << c.branches),
                    branches:   c.branches + 1,
                    score:      c.score,
                    begin:      c.begin,
                    measure:    c.measure
                });
                last = max(last, b_last);
                
                c.pos += len;
                c.branches += 1;
                break;
            
            case 4: // BranchExit
                let skip = item[1];
                c.pos += skip;
                break;
        }
        
        if (c.measure.shrink > self.width) {
            return last; // too full
        }
        
        c.pos += 1;
    }
    
    return last;
}


function maybe_update(self, c, nodes, index) {
    let width = self.width;
    let m = c.measure;
    
    if (width < m.shrink) {
        return false;
    }
    
    var factor;
    if (width == m.width) {
        factor = 1.0;
    } else {
        let delta = width - m.width; // d > 0 => stretch, d < 0 => shrink
        let diff;
        if (delta >= 0.) {
            diff = m.stretch - m.width;
        } else {
            diff = m.width - m.shrink;
        }
        factor = delta / diff;
    };
    let break_score = c.score - factor * factor;
    let other = nodes[index];
    if (other == undefined) {
        nodes[index] = {
            prev:   c.begin,
            path:   c.path,
            factor: factor,
            score:  break_score
        };
    } else if (break_score > other.score) {
        nodes[index] = {
            prev:   c.begin,
            path:   c.path,
            factor: factor,
            score:  break_score
        };
    }
    return true;
}

function show_line() {
    do {
        var line = display_state.lines[display_state.line];
        
        let lineElement = document.createElement("line");
        lineElement.style.top = display_state.y + "px";
        
        let height = line[0];
        let words = line[1];
        for (var word of words) {
            let text = word[0];
            let x = word[1];
            
            let wordElement = document.createElement("span");
            wordElement.appendChild(document.createTextNode(text));
            wordElement.style.left = x + "px";
            lineElement.appendChild(wordElement);
        }
        display_state.y += height;
        
        display_state.target.appendChild(lineElement);
        
        display_state.line += 1;
        
        let time = new Date();
        let dt = time - display_state.time;
        if (dt > 20.) {
            display_state.time = time;
            window.setTimeout(show_line, 0);
            break;
        }
    } while (display_state.line < display_state.lines.length)
}