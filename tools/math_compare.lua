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

local function single_glyph_font_size(box_number)
    local box = assert(tex.box[box_number], "math-size control box is missing")
    local size_sp = nil
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
                if size_sp == nil then
                    size_sp = f.size
                else
                    assert(size_sp == f.size, "math-size control box contains multiple font sizes")
                end
            elseif item.id == hlist_id or item.id == vlist_id then
                visit(item.head)
            end
        end
    end

    visit(box.head)
    assert(glyphs == 1 and size_sp ~= nil, "math-size control box must contain exactly one glyph")
    return size_sp
end

function texpose_measure_math_case(label, box_number, control_box_number, em_sp)
    assert(type(label) == "string" and label:match("^[A-Za-z0-9_-]+$"), "invalid case label")
    assert(type(box_number) == "number", "box_number must be numeric")
    assert(type(control_box_number) == "number", "control_box_number must be numeric")
    assert(type(em_sp) == "number" and em_sp > 0, "em_sp must be positive")

    local box = assert(tex.box[box_number], "math comparison box is missing")
    local root_math_em_sp = single_glyph_font_size(control_box_number)
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
        "|", em_sp,
        "|", stats.glyphs,
        "|", stats.rules,
        "|", stats.hlists,
        "|", stats.vlists,
        "|", stats.max_depth,
        "|", root_math_em_sp,
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
