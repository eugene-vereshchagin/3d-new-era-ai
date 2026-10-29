# 3D New Era AI — connector review kit

What the Claude Connectors Directory and the OpenAI Apps Directory ask for, in one place.
Copy from here into their forms.

- **Server URL:** `https://mcp.3dneweraai.com/mcp` (Streamable HTTP, OAuth 2.1 with
  dynamic client registration and client ID metadata documents, PKCE S256)
- **Public documentation:** <https://3dneweraai.com/connector/>
- **Privacy policy:** <https://3dneweraai.com/privacy/>
- **Terms:** <https://3dneweraai.com/terms/>
- **Support:** <contato@3dneweraai.com>
- **Sign-in:** a one-time link sent by email; no password, no MFA

## Short description

Design your home by talking to your AI: it draws the floor plan, furnishes the rooms,
plans kitchens and wardrobes, and shows you the result as a plan, a 3D view or a photo.

## Long description (connector card, under 2,000 characters)

3D New Era AI turns a conversation into a home design. Ask for an apartment, a room or a
renovation, and your AI draws the walls and rooms, places doors, windows and furniture
from a catalog of real pieces, designs kitchens and built-in wardrobes, and checks the
result against the people who live there: walkways, reach, lighting. You see it as a floor
plan inside the chat, as a 3D view, or as a photo rendered from the project itself — not
an image made up by generative AI.

Every change can be undone, and your projects stay in your account, ready to download and
open in the browser editor or the free desktop app. Everything is free, with no limits,
including high-quality photos; nothing is sold.

## Example prompts (Claude asks for at least 3)

1. "Draw a 60 m² two-bedroom apartment with an open kitchen and a balcony, then show me the floor plan."
2. "Furnish the living room for four people, keep a 90 cm walkway, and render a daylight photo from the sofa."
3. "Design built-in wardrobes for the master bedroom and a kitchen with an island, then check the lighting."
4. "Review the bathroom for ergonomics and fix what fails."
5. "List my projects and open the one called Beach house."

## Test cases (OpenAI asks for at least 5 positive and 3 negative)

Each starts in a new conversation, signed in with the review account.

### Positive — the app should be used

| # | Prompt | Expected |
|---|---|---|
| 1 | "Start a new project called Test flat and draw a 4 × 5 m room." | `file` (`action=new`), then `create` with one closed wall loop; `show_plan` shows a 20 m² room. |
| 2 | "Add a door on the south wall and a double bed against the north wall." | `catalog` finds a door and a bed; `place` puts them; the plan shows both. |
| 3 | "Show me this in 3D." | `render_3d` returns an image of the room with the door and the bed. |
| 4 | "Render a daylight photo of the room." | `render_photo` at draft quality returns a photo. |
| 5 | "Undo the last change." | `edit_history` (`action=undo`); the bed (or the last thing added) is gone from the plan. |
| 6 | "Which projects do I have?" | `projects` lists Test flat with its size and the plan's limits. |
| 7 | "Check this bedroom for ergonomics." | `ergonomics` reports walkways and clearances around the bed. |

### Negative — the app should not be used

| # | Prompt | Why |
|---|---|---|
| 1 | "What is the capital of France?" | Not about designing a space. |
| 2 | "Write a poem about my new house." | Creative writing; nothing to draw. |
| 3 | "Find me a sofa to buy online under 500 dollars." | Shopping; the app has no store and sells nothing. |
| 4 | "Convert 12 square meters to square feet." | Plain arithmetic; no project needed. |

## Review account

An ordinary account, no MFA: sign in at <https://mcp.3dneweraai.com/login> with the review
address given in the form; the sign-in link arrives at that inbox. There is no paid plan, so
any account sees everything, high-quality photos included.

## Checklist

- [x] Every tool has a `title` and correct `readOnlyHint` / `destructiveHint` /
      `openWorldHint` (tests `every_tool_has_hints`, `reads_change_nothing`)
- [x] Reads and writes in separate tools; names under 64 characters
- [x] Errors say what to do next (for example, a limit error names the quality that still works; no limits are set today)
- [x] Descriptions give no orders to the model and carry no promotion or upsell
- [x] No selling, prices or checkout links inside the chat; no tool takes payments
- [x] Everything is free, with no limits; nothing is sold
- [x] Minimal data: email, projects and usage; never the conversation
- [x] Privacy policy and terms published at 3dneweraai.com
- [x] Public connector documentation
- [x] Example prompts and test cases (above)
- [x] Images come from deterministic rendering of the project, said on the connector page
- [x] The widget declares its CSP: it loads nothing from the network
- [x] Screenshots of the widget ([images/connector/](images/connector/))
- [ ] Review account (any account; there is no paid plan)
- [ ] Claude Team organization (to submit to the Connectors Directory)
- [ ] Verified identity on the OpenAI Platform
