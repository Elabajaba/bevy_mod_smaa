// Copyright (C) 2013 Jorge Jimenez (jorge@iryoku.com)
// Copyright (C) 2013 Jose I. Echevarria (joseignacioechevarria@gmail.com)
// Copyright (C) 2013 Belen Masia (bmasia@unizar.es)
// Copyright (C) 2013 Fernando Navarro (fernandn@microsoft.com)
// Copyright (C) 2013 Diego Gutierrez (diegog@unizar.es)
// Permission is hereby granted, free of charge, to any person obtaining a copy
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
// of the Software, and to permit persons to whom the Software is furnished to
// do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software. As clarification, there
// is no requirement that the copyright notice and permission be included in
// binary distributions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//-----------------------------------------------------------------------------
// Diagonal Search Functions

#ifndef SMAA_DISABLE_DIAG_DETECTION

// Allows to decode two binary values from a bilinear-filtered access.
fn SMAADecodeDiagBilinearAccess2v(e: vec2<f32>) -> vec2<f32> {
    // Bilinear access for fetching 'e' have a 0.25 offset, and we are
    // interested in the R and G edges:
    //
    // +---G---+-------+
    // |   x o R   x   |
    // +-------+-------+
    //
    // Then, if one of these edge is enabled:
    //   Red:   (0.75 * X + 0.25 * 1) => 0.25 or 1.0
    //   Green: (0.75 * 1 + 0.25 * X) => 0.75 or 1.0
    //
    // This function will unpack the values (mad + mul + round):
    // wolframalpha.com: round(x * abs(5 * x - 5 * 0.75)) plot 0 to 1
    e.r = e.r * abs(5.0 * e.r - 5.0 * 0.75);
    return round(e);
}

fn SMAADecodeDiagBilinearAccess4v(e: vec4<f32>) -> vec4<f32> {
    e.rb = e.rb * abs(5.0 * e.rb - 5.0 * 0.75);
    return round(e);
}

// These functions allows to perform diagonal pattern searches.
fn SMAASearchDiag1v(SMAATexture2D(edgesTex), texcoord: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    let t = vec3<f32>(SMAA_RT_METRICS.xy, 1.0);
    while (coord.z < f32(SMAA_MAX_SEARCH_STEPS_DIAG - 1) &&
           coord.w > 0.9) {
        coord.xyz = fma(t, vec3<f32>(dir, 1.0), coord.xyz);
        e = SMAASampleLevelZero(edgesTex, coord.xy).rg;
        coord.w = dot(e, vec2<f32>(0.5, 0.5));
    }
    return coord.zw;
}

fn SMAASearchDiag2v(SMAATexture2D(edgesTex), texcoord: vec2<f32>, dir: vec2<f32>, e: ptr<function, vec2<f32>>) -> vec2<f32> {
    var coord = vec4<f32>(texcoord, -1.0, 1.0);
    coord.x += 0.25 * SMAA_RT_METRICS.x; // See @SearchDiag2Optimization
    let t = vec3<f32>(SMAA_RT_METRICS.xy, 1.0);
    while (coord.z < f32(SMAA_MAX_SEARCH_STEPS_DIAG - 1) &&
           coord.w > 0.9) {
        coord.xyz = fma(t, vec3<f32>(dir, 1.0), coord.xyz);

        // @SearchDiag2Optimization
        // Fetch both edges at once using bilinear filtering:
        e = SMAASampleLevelZero(edgesTex, coord.xy).rg;
        e = SMAADecodeDiagBilinearAccess(e);

        // Non-optimized version:
        // e.g = SMAASampleLevelZero(edgesTex, coord.xy).g;
        // e.r = SMAASampleLevelZeroOffset(edgesTex, coord.xy, int2(1, 0)).r;

        coord.w = dot(e, vec2<f32>(0.5, 0.5));
    }
    return coord.zw;
}

// Similar to SMAAArea, this calculates the area corresponding to a certain diagonal distance and crossing edges 'e'.
fn SMAAAreaDiag(SMAATexture2D(areaTex), dist: vec2<f32>, e: vec2<f32>, offset: f32) -> vec2<f32> {
    var texcoord = fma(vec2<f32>(SMAA_AREATEX_MAX_DISTANCE_DIAG, SMAA_AREATEX_MAX_DISTANCE_DIAG), e, dist);

    // We do a scale and bias for mapping to texel space:
    texcoord = fma(SMAA_AREATEX_PIXEL_SIZE, texcoord, 0.5 * SMAA_AREATEX_PIXEL_SIZE);

    // Diagonal areas are on the second half of the texture:
    texcoord.x += 0.5;

    // Move to proper place, according to the subpixel offset:
    texcoord.y += SMAA_AREATEX_SUBTEX_SIZE * offset;

    // Do it!
    return SMAA_AREATEX_SELECT(SMAASampleLevelZero(areaTex, texcoord));
}

// This searches for diagonal patterns and returns the corresponding weights.
fn SMAACalculateDiagWeights(SMAATexture2D(edgesTex), SMAATexture2D(areaTex), texcoord: vec2<f32>, e: vec2<f32>, subsampleIndices: vec4<f32>) -> vec2<f32> {
    var weights = vec2<f32>(0.0, 0.0);

    // Search for the line ends:
    var d: vec4<f32>;
    var end: vec2<f32>;
    if (e.r > 0.0) {
        d.xz = SMAASearchDiag1(SMAATexturePass2D(edgesTex), texcoord, vec2<f32>(-1.0,  1.0), end);
        d.x += float(end.y > 0.9);
    } else
        d.xz = vec2<f32>(0.0, 0.0);
    d.yw = SMAASearchDiag1(SMAATexturePass2D(edgesTex), texcoord, vec2<f32>(1.0, -1.0), end);

    if (d.x + d.y > 2.0) { // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        let coords: vec4<f32> = fma(vec4<f32>(-d.x + 0.25, d.x, d.y, -d.y - 0.25), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        var c: vec4<f32>;
        c.xy = SMAASampleLevelZeroOffset(edgesTex, coords.xy, vec2<i32>(-1,  0)).rg;
        c.zw = SMAASampleLevelZeroOffset(edgesTex, coords.zw, vec2<i32>( 1,  0)).rg;
        c.yxwz = SMAADecodeDiagBilinearAccess(c.xyzw);

        // Non-optimized version:
        // vec4<f32> coords = mad(vec4<f32>(-d.x, d.x, d.y, -d.y), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        // vec4<f32> c;
        // c.x = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2(-1,  0)).g;
        // c.y = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2( 0,  0)).r;
        // c.z = SMAASampleLevelZeroOffset(edgesTex, coords.zw, int2( 1,  0)).g;
        // c.w = SMAASampleLevelZeroOffset(edgesTex, coords.zw, int2( 1, -1)).r;

        // Merge crossing edges at each side into a single value:
        var cc: vec2<f32> = fma(vec2<f32>(2.0, 2.0), c.xz, c.yw);

        // Remove the crossing edge if we didn't found the end of the line:
        SMAAMovc(vec2<bool>(step(0.9, d.zw)), cc, vec2<f32>(0.0, 0.0));

        // Fetch the areas for this line:
        weights += SMAAAreaDiag(SMAATexturePass2D(areaTex), d.xy, cc, subsampleIndices.z);
    }

    // Search for the line ends:
    d.xz = SMAASearchDiag2(SMAATexturePass2D(edgesTex), texcoord, vec2<f32>(-1.0, -1.0), end);
    if (SMAASampleLevelZeroOffset(edgesTex, texcoord, int2(1, 0)).r > 0.0) {
        d.yw = SMAASearchDiag2(SMAATexturePass2D(edgesTex), texcoord, vec2<f32>(1.0, 1.0), end);
        d.y += f32(end.y > 0.9);
    } else
        d.yw = vec2<f32>(0.0, 0.0);

    if (d.x + d.y > 2.0) { // d.x + d.y + 1 > 3
        // Fetch the crossing edges:
        let coords: vec4<f32> = fma(vec4<f32>(-d.x, -d.x, d.y, d.y), SMAA_RT_METRICS.xyxy, texcoord.xyxy);
        var c: vec4<f32>;
        c.x  = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2(-1,  0)).g;
        c.y  = SMAASampleLevelZeroOffset(edgesTex, coords.xy, int2( 0, -1)).r;
        c.zw = SMAASampleLevelZeroOffset(edgesTex, coords.zw, int2( 1,  0)).gr;
        var cc: vec2<f32> = fma(vec2<f32>(2.0, 2.0), c.xz, c.yw);

        // Remove the crossing edge if we didn't found the end of the line:
        SMAAMovc(vec2<bool>(step(0.9, d.zw)), cc, vec2<f32>(0.0, 0.0));

        // Fetch the areas for this line:
        weights += SMAAAreaDiag(SMAATexturePass2D(areaTex), d.xy, cc, subsampleIndices.w).gr;
    }

    return weights;
}
#endif


//-----------------------------------------------------------------------------
// Horizontal/Vertical Search Functions

// This allows to determine how much length should we add in the last step
// of the searches. It takes the bilinearly interpolated edge (see 
// @PSEUDO_GATHER4), and adds 0, 1 or 2, depending on which edges and
// crossing edges are active.
fn SMAASearchLength(SMAATexture2D(searchTex), e: vec2<f32>, offset: f32) -> f32 {
    // The texture is flipped vertically, with left and right cases taking half
    // of the space horizontally:
    var scale: vec2<f32> = SMAA_SEARCHTEX_SIZE * vec2<f32>(0.5, -1.0);
    var bias: vec2<f32> = SMAA_SEARCHTEX_SIZE * vec2<f32>(offset, 1.0);

    // Scale and bias to access texel centers:
    scale += vec2<f32>(-1.0,  1.0);
    bias  += vec2<f32>( 0.5, -0.5);

    // Convert from pixel coordinates to texcoords:
    // (We use SMAA_SEARCHTEX_PACKED_SIZE because the texture is cropped)
    scale *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;
    bias *= 1.0 / SMAA_SEARCHTEX_PACKED_SIZE;

    // Lookup the search texture:
    return SMAA_SEARCHTEX_SELECT(SMAASampleLevelZero(searchTex, fma(scale, e, bias)));
}

// Horizontal/vertical search functions for the 2nd pass.
fn SMAASearchXLeft(SMAATexture2D(edgesTex), SMAATexture2D(searchTex), texcoord: vec2<f32>, end: f32) -> f32 {
    // @PSEUDO_GATHER4
    // This texcoord has been offset by (-0.25, -0.125) in the vertex shader to
    // sample between edge, thus fetching four edges in a row.
    // Sampling with different offsets in each direction allows to disambiguate
    // which edges are active from the four fetched ones.
    var e = vec2<f32>(0.0, 1.0);
    while (texcoord.x > end && 
           e.g > 0.8281 && // Is there some edge not activated?
           e.r == 0.0) { // Or is there a crossing edge that breaks the line?
        e = SMAASampleLevelZero(edgesTex, texcoord).rg;
        texcoord = fma(-vec2<f32>(2.0, 0.0), SMAA_RT_METRICS.xy, texcoord);
    }

    float offset = fma(-(255.0 / 127.0), SMAASearchLength(SMAATexturePass2D(searchTex), e, 0.0), 3.25);
    return fma(SMAA_RT_METRICS.x, offset, texcoord.x);

    // Non-optimized version:
    // We correct the previous (-0.25, -0.125) offset we applied:
    // texcoord.x += 0.25 * SMAA_RT_METRICS.x;

    // The searches are bias by 1, so adjust the coords accordingly:
    // texcoord.x += SMAA_RT_METRICS.x;

    // Disambiguate the length added by the last step:
    // texcoord.x += 2.0 * SMAA_RT_METRICS.x; // Undo last step
    // texcoord.x -= SMAA_RT_METRICS.x * (255.0 / 127.0) * SMAASearchLength(SMAATexturePass2D(searchTex), e, 0.0);
    // return fma(SMAA_RT_METRICS.x, offset, texcoord.x);
}

fn SMAASearchXRight(SMAATexture2D(edgesTex), SMAATexture2D(searchTex), texcoord: vec2<f32>, end: f32) -> f32 {
    var e = vec2<f32>(0.0, 1.0);
    while (texcoord.x < end && 
           e.g > 0.8281 && // Is there some edge not activated?
           e.r == 0.0) { // Or is there a crossing edge that breaks the line?
        e = SMAASampleLevelZero(edgesTex, texcoord).rg;
        texcoord = fma(vec2<f32>(2.0, 0.0), SMAA_RT_METRICS.xy, texcoord);
    }
    let offset: f32 = fma(-(255.0 / 127.0), SMAASearchLength(SMAATexturePass2D(searchTex), e, 0.5), 3.25);
    return fma(-SMAA_RT_METRICS.x, offset, texcoord.x);
}

fn SMAASearchYUp(SMAATexture2D(edgesTex), SMAATexture2D(searchTex), texcoord: vec2<f32>, end: f32) -> f32 {
    var e = vec2<f32>(1.0, 0.0);
    while (texcoord.y > end && 
           e.r > 0.8281 && // Is there some edge not activated?
           e.g == 0.0) { // Or is there a crossing edge that breaks the line?
        e = SMAASampleLevelZero(edgesTex, texcoord).rg;
        texcoord = fma(-vec2<f32>(0.0, 2.0), SMAA_RT_METRICS.xy, texcoord);
    }
    let offset: f32 = fma(-(255.0 / 127.0), SMAASearchLength(SMAATexturePass2D(searchTex), e.gr, 0.0), 3.25);
    return fma(SMAA_RT_METRICS.y, offset, texcoord.y);
}

fn SMAASearchYDown(SMAATexture2D(edgesTex), SMAATexture2D(searchTex), texcoord: vec2<f32>, end: f32) -> f32 {
    var e = vec2<f32>(1.0, 0.0);
    while (texcoord.y < end && 
           e.r > 0.8281 && // Is there some edge not activated?
           e.g == 0.0) { // Or is there a crossing edge that breaks the line?
        e = SMAASampleLevelZero(edgesTex, texcoord).rg;
        texcoord = fma(vec2<f32>(0.0, 2.0), SMAA_RT_METRICS.xy, texcoord);
    }
    let offset: f32 = fma(-(255.0 / 127.0), SMAASearchLength(SMAATexturePass2D(searchTex), e.gr, 0.5), 3.25);
    return fma(-SMAA_RT_METRICS.y, offset, texcoord.y);
}

// Ok, we have the distance and both crossing edges. So, what are the areas
// at each side of current edge?
fn SMAAArea(SMAATexture2D(areaTex), dist: vec2<f32>, e1: f32, e2: f32, offset: f32) -> vec2<f32> {
    // Rounding prevents precision errors of bilinear filtering:
    var texcoord: vec2<f32> = fma(vec2<f32>(SMAA_AREATEX_MAX_DISTANCE, SMAA_AREATEX_MAX_DISTANCE), round(4.0 * vec2<f32>(e1, e2)), dist);
    
    // We do a scale and bias for mapping to texel space:
    texcoord = fma(SMAA_AREATEX_PIXEL_SIZE, texcoord, 0.5 * SMAA_AREATEX_PIXEL_SIZE);

    // Move to proper place, according to the subpixel offset:
    texcoord.y = fma(SMAA_AREATEX_SUBTEX_SIZE, offset, texcoord.y);

    // Do it!
    return SMAA_AREATEX_SELECT(SMAASampleLevelZero(areaTex, texcoord));
}


//-----------------------------------------------------------------------------
// Corner Detection Functions

fn SMAADetectHorizontalCornerPattern(SMAATexture2D(edgesTex), weights: ptr<function, vec2<f32>>, texcoord: vec4<f32>, d: vec2<f32>) {
    #ifndef SMAA_DISABLE_CORNER_DETECTION
    let leftRight = step(d.xy, d.yx);
    var rounding: vec2<f32> = (1.0 - SMAA_CORNER_ROUNDING_NORM) * leftRight;

    rounding /= leftRight.x + leftRight.y; // Reduce blending for pixels in the center of a line.

    var factor = vec2<f32>(1.0, 1.0);
    factor.x -= rounding.x * SMAASampleLevelZeroOffset(edgesTex, texcoord.xy, int2(0,  1)).r;
    factor.x -= rounding.y * SMAASampleLevelZeroOffset(edgesTex, texcoord.zw, int2(1,  1)).r;
    factor.y -= rounding.x * SMAASampleLevelZeroOffset(edgesTex, texcoord.xy, int2(0, -2)).r;
    factor.y -= rounding.y * SMAASampleLevelZeroOffset(edgesTex, texcoord.zw, int2(1, -2)).r;

    weights *= saturate(factor);
    #endif
}

fn SMAADetectVerticalCornerPattern(SMAATexture2D(edgesTex), weights: ptr<function, vec2<f32>>, texcoord: vec4<f32>, d: vec2<f32>) {
    #ifndef SMAA_DISABLE_CORNER_DETECTION
    let leftRight = step(d.xy, d.yx);
    var rounding: vec2<f32> = (1.0 - SMAA_CORNER_ROUNDING_NORM) * leftRight;

    rounding /= leftRight.x + leftRight.y;

    var factor = vec2<f32>(1.0, 1.0);
    factor.x -= rounding.x * SMAASampleLevelZeroOffset(edgesTex, texcoord.xy, int2( 1, 0)).g;
    factor.x -= rounding.y * SMAASampleLevelZeroOffset(edgesTex, texcoord.zw, int2( 1, 1)).g;
    factor.y -= rounding.x * SMAASampleLevelZeroOffset(edgesTex, texcoord.xy, int2(-2, 0)).g;
    factor.y -= rounding.y * SMAASampleLevelZeroOffset(edgesTex, texcoord.zw, int2(-2, 1)).g;

    weights *= saturate(factor);
    #endif
}