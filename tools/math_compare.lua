-- Test-only LuaTeX instrumentation for tools/verify.py math.
-- It records final LuaTeX math-box geometry and positioned glyph/rule primitives
-- without parsing PDF output or changing TeXpose.

local result_path = assert(texpose_math_result_file, "texpose_math_result_file is not set")
local out = assert(io.open(result_path, "w"))

local glyph_id = node.id("glyph")
local rule_id = node.id("rule")
local hlist_id = node.id("hlist")
local vlist_id = node.id("vlist")
local glue_id = node.id("glue")
local kern_id = node.id("kern")
local math_id = node.id("math")
local running_dimension = -1073741824

local function inspect_list(head, depth, stats)
    if not head then
        return
    end
    if depth > stats.max_depth then
        stats.max_depth = depth
    end
    for item in node.traverse(head) do
        if item.id == glyph_id then
            stats.glyphs = stats.glyphs + 1
        elseif item.id == rule_id then
            stats.rules = stats.rules + 1
        elseif item.id == hlist_id then
            stats.hlists = stats.hlists + 1
            inspect_list(item.head, depth + 1, stats)
        elseif item.id == vlist_id then
            stats.vlists = stats.vlists + 1
            inspect_list(item.head, depth + 1, stats)
        end
    end
end

local function single_glyph_info(box_number)
    local box = assert(tex.box[box_number], "math-size control box is missing")
    local size_sp = nil
    local subfont = nil
    local glyphs = 0

    local function visit(head)
        if not head then
            return
        end
        for item in node.traverse(head) do
            if item.id == glyph_id then
                glyphs = glyphs + 1
                local f = assert(font.getfont(item.font), "math-size control font is missing")
                assert(type(f.size) == "number" and f.size > 0, "invalid math-size control font size")
                local current_subfont = f.subfont or 0
                assert(type(current_subfont) == "number" and current_subfont >= 0, "invalid math-size control subfont")
                if size_sp == nil then
                    size_sp = f.size
                    subfont = current_subfont
                else
                    assert(size_sp == f.size, "math-size control box contains multiple font sizes")
                    assert(subfont == current_subfont, "math-size control box contains multiple collection faces")
                end
            elseif item.id == hlist_id or item.id == vlist_id then
                visit(item.head)
            end
        end
    end

    visit(box.head)
    assert(glyphs == 1 and size_sp ~= nil and subfont ~= nil, "math-size control box must contain exactly one glyph")
    return size_sp, subfont
end

local function require_tlt(box)
    assert(box.dir == "TLT", "positioned math trace only supports TLT lists")
end

local function glue_advance(box, item, state)
    assert(item.leader == nil, "positioned math trace does not support leaders")
    local old_g = state.cur_g
    if box.glue_sign == 1 and item.stretch_order == box.glue_order then
        state.cur_glue = state.cur_glue + item.stretch
    elseif box.glue_sign == 2 and item.shrink_order == box.glue_order then
        state.cur_glue = state.cur_glue - item.shrink
    end
    local glue_temp = box.glue_set * state.cur_glue
    if glue_temp > 1000000000 then
        glue_temp = 1000000000
    elseif glue_temp < -1000000000 then
        glue_temp = -1000000000
    end
    state.cur_g = tex.round(glue_temp)
    return item.width - old_g + state.cur_g
end

local function push_glyph(trace, item, x, baseline)
    local f = assert(font.getfont(item.font), "trace glyph font is missing")
    local character = assert(f.characters[item.char], "trace glyph character is missing")
    local glyph_index = character.index
    assert(type(glyph_index) == "number" and glyph_index >= 0, "trace glyph index is missing")
    assert(type(f.size) == "number" and f.size > 0, "trace glyph font size is invalid")
    trace[#trace + 1] = {
        kind = "G",
        glyph_id = glyph_index,
        x = x + (item.xoffset or 0),
        baseline = baseline + (item.yoffset or 0),
        font_size = f.size,
    }
end

local function push_rule(trace, x, bottom, width, height)
    if width > 0 and height > 0 then
        trace[#trace + 1] = {
            kind = "R",
            x = x,
            bottom = bottom,
            width = width,
            height = height,
        }
    end
end

local trace_hlist
local trace_vlist

trace_hlist = function(box, x, baseline, trace)
    require_tlt(box)
    local cursor = x
    local glue_state = { cur_glue = 0, cur_g = 0 }
    for item in node.traverse(box.head) do
        if item.id == glyph_id then
            push_glyph(trace, item, cursor, baseline)
            cursor = cursor + item.width
        elseif item.id == hlist_id or item.id == vlist_id then
            if item.head then
                local child_baseline = baseline - item.shift
                if item.id == hlist_id then
                    trace_hlist(item, cursor, child_baseline, trace)
                else
                    trace_vlist(item, cursor, child_baseline, trace)
                end
            end
            cursor = cursor + item.width
        elseif item.id == rule_id then
            local height = item.height == running_dimension and box.height or item.height
            local depth = item.depth == running_dimension and box.depth or item.depth
            local total = height + depth
            push_rule(trace, cursor, baseline - depth, item.width, total)
            cursor = cursor + item.width
        elseif item.id == glue_id then
            cursor = cursor + glue_advance(box, item, glue_state)
        elseif item.id == kern_id then
            cursor = cursor + item.kern
        elseif item.id == math_id then
            cursor = cursor + (item.width or 0)
        else
            local kind = node.type(item.id)
            if kind ~= "penalty" and kind ~= "whatsit" then
                error("unsupported hlist node in positioned math trace: " .. tostring(kind))
            end
        end
    end
end

trace_vlist = function(box, x, baseline, trace)
    require_tlt(box)
    local cursor_y = baseline + box.height
    local glue_state = { cur_glue = 0, cur_g = 0 }
    for item in node.traverse(box.head) do
        if item.id == hlist_id or item.id == vlist_id then
            if item.head then
                cursor_y = cursor_y - item.height
                local child_x = x + item.shift
                if item.id == hlist_id then
                    trace_hlist(item, child_x, cursor_y, trace)
                else
                    trace_vlist(item, child_x, cursor_y, trace)
                end
                cursor_y = cursor_y - item.depth
            else
                cursor_y = cursor_y - item.height - item.depth
            end
        elseif item.id == rule_id then
            local width = item.width == running_dimension and box.width or item.width
            local total = item.height + item.depth
            cursor_y = cursor_y - total
            push_rule(trace, x, cursor_y, width, total)
        elseif item.id == glue_id then
            cursor_y = cursor_y - glue_advance(box, item, glue_state)
        elseif item.id == kern_id then
            cursor_y = cursor_y - item.kern
        else
            local kind = node.type(item.id)
            if kind ~= "penalty" and kind ~= "whatsit" then
                error("unsupported vlist node in positioned math trace: " .. tostring(kind))
            end
        end
    end
end

local function write_trace(label, trace)
    for index, primitive in ipairs(trace) do
        if primitive.kind == "G" then
            out:write(
                "TRACE|", label,
                "|", index - 1,
                "|G|", primitive.glyph_id,
                "|", primitive.x,
                "|", primitive.baseline,
                "|", primitive.font_size,
                "\n"
            )
        else
            out:write(
                "TRACE|", label,
                "|", index - 1,
                "|R|", primitive.x,
                "|", primitive.bottom,
                "|", primitive.width,
                "|", primitive.height,
                "\n"
            )
        end
    end
end

local function hex(value)
    assert(type(value) == "string", "fingerprint field must be a string")
    return (value:gsub(".", function(c)
        return string.format("%02x", string.byte(c))
    end))
end

function texpose_write_fingerprint(latex, unicode_math, fontspec, amsmath, font_sha256, face_index, profile, revision, census_sha256, alias_census_sha256)
    assert(type(font_sha256) == "string" and font_sha256:match("^[0-9a-f]+$"), "invalid font hash")
    assert(type(face_index) == "number" and face_index >= 0, "invalid face index")
    assert(type(profile) == "string" and profile:match("^[A-Za-z0-9_-]+$"), "invalid profile")
    assert(type(revision) == "string" and revision:match("^[A-Za-z0-9_.-]+$"), "invalid revision")
    assert(type(census_sha256) == "string" and census_sha256:match("^[0-9a-f]+$"), "invalid census hash")
    assert(type(alias_census_sha256) == "string" and alias_census_sha256:match("^[0-9a-f]+$"), "invalid alias census hash")
    local engine = status.banner or ""
    local distribution = engine:match("%((TeX Live [^)]+)%)")
        or engine:match("%((MiKTeX [^)]+)%)")
        or ""
    out:write(
        "FINGERPRINT|", hex(engine),
        "|", hex(distribution),
        "|", hex(latex),
        "|", hex(unicode_math),
        "|", hex(fontspec),
        "|", hex(amsmath),
        "|", font_sha256,
        "|", face_index,
        "|", profile,
        "|", revision,
        "|", census_sha256,
        "|", alias_census_sha256,
        "\n"
    )
    out:flush()
end

function texpose_measure_math_case(label, box_number, text_box_number, script_box_number, scriptscript_box_number)
    assert(type(label) == "string" and label:match("^[A-Za-z0-9_-]+$"), "invalid case label")
    assert(type(box_number) == "number", "box_number must be numeric")
    assert(type(text_box_number) == "number", "text_box_number must be numeric")
    assert(type(script_box_number) == "number", "script_box_number must be numeric")
    assert(type(scriptscript_box_number) == "number", "scriptscript_box_number must be numeric")

    local box = assert(tex.box[box_number], "math comparison box is missing")
    require_tlt(box)
    local text_size_sp, text_subfont = single_glyph_info(text_box_number)
    local script_size_sp, script_subfont = single_glyph_info(script_box_number)
    local scriptscript_size_sp, scriptscript_subfont = single_glyph_info(scriptscript_box_number)
    assert(text_subfont == script_subfont and text_subfont == scriptscript_subfont, "math styles selected different collection faces")

    local stats = {
        glyphs = 0,
        rules = 0,
        hlists = 0,
        vlists = 0,
        max_depth = 0,
    }
    inspect_list(box.head, 0, stats)

    local trace = {}
    trace_hlist(box, 0, 0, trace)

    out:write(
        "CASE|", label,
        "|", box.width,
        "|", box.height,
        "|", box.depth,
        "|", text_size_sp,
        "|", script_size_sp,
        "|", scriptscript_size_sp,
        "|", stats.glyphs,
        "|", stats.rules,
        "|", stats.hlists,
        "|", stats.vlists,
        "|", stats.max_depth,
        "|", #trace,
        "|", text_subfont,
        "\n"
    )
    write_trace(label, trace)
    out:flush()
end

luatexbase.add_to_callback(
    "stop_run",
    function()
        if out then
            out:close()
            out = nil
        end
    end,
    "texpose-math-comparison-close"
)
