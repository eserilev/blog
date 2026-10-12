import Logbook.Paint.Draw
import Logbook.Paint.Fill
import Logbook.Paint.Flip
import Logbook.Paint.Rotate
import Logbook.Paint.Total
open Aeneas Aeneas.Std Result logbook_paint

/-! P10 in one place (spec 7.6): every public function of `logbook-paint` returns `ok`
for every input, and a function that changes a canvas does not change an invalid one. -/

namespace Logbook.Paint

/-- P10: no public function panics, overflows, or reads out of bounds, for any input. -/
theorem no_panic :
    (∀ c, valid c ⦃ _ => True ⦄) ∧
    (∀ v, clamp_coord v ⦃ _ => True ⦄) ∧
    (∀ n, clamp_dim n ⦃ _ => True ⦄) ∧
    (∀ w h color, new_canvas w h color ⦃ _ => True ⦄) ∧
    (∀ c x y, index c x y ⦃ _ => True ⦄) ∧
    (∀ c x y, logbook_paint.get c x y ⦃ _ => True ⦄) ∧
    (∀ c x y color, logbook_paint.set c x y color ⦃ _ => True ⦄) ∧
    (∀ x y w h cw ch, clip_rect x y w h cw ch ⦃ _ => True ⦄) ∧
    (∀ c x y w h color, fill_rect c x y w h color ⦃ _ => True ⦄) ∧
    (∀ c x y w h lw color, rect_outline c x y w h lw color ⦃ _ => True ⦄) ∧
    (∀ x0 y0 x1 y1, line_points x0 y0 x1 y1 ⦃ _ => True ⦄) ∧
    (∀ c x0 y0 x1 y1 width color, line c x0 y0 x1 y1 width color ⦃ _ => True ⦄) ∧
    (∀ c x y color, flood_fill c x y color ⦃ _ => True ⦄) ∧
    (∀ c x y w h f t, color_replace c x y w h f t ⦃ _ => True ⦄) ∧
    (∀ c, flip_h c ⦃ _ => True ⦄) ∧
    (∀ c, flip_v c ⦃ _ => True ⦄) ∧
    (∀ c, rotate90 c ⦃ _ => True ⦄) ∧
    (∀ c, invert c ⦃ _ => True ⦄) ∧
    (∀ c, to_black_white c ⦃ _ => True ⦄) ∧
    (∀ c x y w h, copy_region c x y w h ⦃ _ => True ⦄) ∧
    (∀ dst src x y use_key key, blit dst src x y use_key key ⦃ _ => True ⦄) ∧
    (∀ c w h bg, resize c w h bg ⦃ _ => True ⦄) ∧
    (∀ c nw nh, stretch c nw nh ⦃ _ => True ⦄) ∧
    (∀ c num bg, skew_x c num bg ⦃ _ => True ⦄) ∧
    (∀ c num bg, skew_y c num bg ⦃ _ => True ⦄) := by
  refine ⟨fun c => ?_, fun v => ?_, fun n => ?_, fun w h color => ?_, fun c x y => ?_,
    fun c x y => ?_, fun c x y color => ?_, fun x y w h cw ch => ?_, fun c x y w h color => ?_,
    fun c x y w h lw color => ?_, fun x0 y0 x1 y1 => ?_, fun c x0 y0 x1 y1 width color => ?_,
    fun c x y color => ?_, fun c x y w h f t => ?_, fun c => ?_, fun c => ?_, fun c => ?_,
    fun c => ?_, fun c => ?_, fun c x y w h => ?_, fun dst src x y use_key key => ?_,
    fun c w h bg => ?_, fun c nw nh => ?_, fun c num bg => ?_, fun c num bg => ?_⟩
  all_goals first
    | exact WP.spec_mono (valid_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (clamp_coord_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (clamp_dim_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (new_canvas_spec _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (index_spec _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (get_spec _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (set_spec _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (clip_rect_spec _ _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (fill_rect_spec _ _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (rect_outline_spec _ _ _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (line_points_spec _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (line_spec _ _ _ _ _ _ _) (fun _ _ => trivial)
    | exact flood_fill_ok _ _ _ _
    | exact WP.spec_mono (color_replace_spec _ _ _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (flip_h_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (flip_v_spec _) (fun _ _ => trivial)
    | exact rotate90_total _
    | exact WP.spec_mono (invert_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (to_black_white_spec _) (fun _ _ => trivial)
    | exact WP.spec_mono (copy_region_spec _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (blit_spec _ _ _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (resize_spec _ _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (stretch_spec _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (skew_x_spec _ _ _) (fun _ _ => trivial)
    | exact WP.spec_mono (skew_y_spec _ _ _) (fun _ _ => trivial)

/-- P10: a function that takes `&mut Canvas` does not change a canvas that is not valid. -/
theorem invalid_unchanged (c : Canvas) (hc : ¬ Valid c) :
    (∀ x y color, logbook_paint.set c x y color ⦃ r => r = c ⦄) ∧
    (∀ x y w h color, fill_rect c x y w h color ⦃ r => r = c ⦄) ∧
    (∀ x y w h lw color, rect_outline c x y w h lw color ⦃ r => r = c ⦄) ∧
    (∀ x0 y0 x1 y1 width color, line c x0 y0 x1 y1 width color ⦃ r => r = c ⦄) ∧
    (∀ x y color, flood_fill c x y color ⦃ r => r = c ⦄) ∧
    (∀ x y w h f t, color_replace c x y w h f t ⦃ r => r = c ⦄) ∧
    flip_h c ⦃ r => r = c ⦄ ∧
    flip_v c ⦃ r => r = c ⦄ ∧
    invert c ⦃ r => r = c ⦄ ∧
    to_black_white c ⦃ r => r = c ⦄ ∧
    (∀ src x y use_key key, blit c src x y use_key key ⦃ r => r = c ⦄) := by
  refine ⟨fun x y color => ?_, fun x y w h color => ?_, fun x y w h lw color => ?_,
    fun x0 y0 x1 y1 width color => ?_, fun x y color => flood_fill_invalid c x y color hc,
    fun x y w h f t => ?_, ?_, ?_, ?_, ?_, fun src x y use_key key => ?_⟩
  · exact WP.spec_mono (set_spec c x y color) (fun r h => h.2.2.2 (fun h' => hc h'.1))
  · exact WP.spec_mono (fill_rect_spec c x y w h color) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (rect_outline_spec c x y w h lw color) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (line_spec c x0 y0 x1 y1 width color) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (color_replace_spec c x y w h f t) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (flip_h_spec c) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (flip_v_spec c) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (invert_spec c) (fun r h => h.2.2.2.2 hc)
  · exact WP.spec_mono (to_black_white_spec c) (fun r h => h.2.2.2 hc)
  · exact WP.spec_mono (blit_spec c src x y use_key key) (fun r h => h.2.2.2 hc)

end Logbook.Paint
