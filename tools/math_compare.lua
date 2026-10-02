-- Test-only LuaTeX instrumentation for tools/verify.py math.
-- It records final LuaTeX math-box geometry without parsing PDF output or changing TeXpose.

local result_path = assert(texpose_math_result_file, "texpose_math_result_file is not set")
local out = assert(io.open(result_path, "w"))

local glyph_id = node.id("glyph")
local rule_id = node.id("rule")
local hlist_id = node.id("hlist")
local vlist_id = node.id("vlist")

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
    local glyph_index = nil
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
                local character = assert(f.characters[item.char], "math-size control character is missing")
                assert(type(character.index) == "number" and character.index >= 0, "math-size control glyph index is missing")
                local current_subfont = f.subfont or 0
                assert(type(current_subfont) == "number" and current_subfont >= 0, "invalid math-size control subfont")
                if size_sp == nil then
                    size_sp = f.size
                    glyph_index = character.index
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
    assert(glyphs == 1 and size_sp ~= nil and glyph_index ~= nil and subfont ~= nil, "math-size control box must contain exactly one glyph")
    return size_sp, glyph_index, subfont
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
    local text_size_sp, text_glyph_index, text_subfont = single_glyph_info(text_box_number)
    local script_size_sp, _, script_subfont = single_glyph_info(script_box_number)
    local scriptscript_size_sp, _, scriptscript_subfont = single_glyph_info(scriptscript_box_number)
    assert(text_subfont == script_subfont and text_subfont == scriptscript_subfont, "math styles selected different collection faces")

    local stats = {
        glyphs = 0,
        rules = 0,
        hlists = 0,
        vlists = 0,
        max_depth = 0,
    }
    inspect_list(box.head, 0, stats)

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
        "|", text_glyph_index,
        "|", text_subfont,
        "\n"
    )
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
