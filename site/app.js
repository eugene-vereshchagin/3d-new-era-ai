(() => {
  "use strict";
  document.documentElement.classList.add("js");

  const REPO = "leandrodaf/3d-new-era-ai";
  const MCP = "http://127.0.0.1:7878/mcp";
  const RAW = `https://raw.githubusercontent.com/${REPO}/main/scripts`;

  // The Product Hunt badge in the footer. It only exists after the launch is
  // live and the post has an id — fill `post` in and the footer shows the
  // official embed; leave it empty and there is no badge and no request.
  const PRODUCT_HUNT = { post: "", slug: "3d-new-era-ai" };

  const EN = {
    "hero.try": "Open in the browser",
    "nav.try": "Open in the browser",
    "q7": "Does it run in a browser?",
    "a7": "The full editor compiles to WebAssembly and runs in a tab. <b>Chrome and Edge</b> are the supported browsers (and Safari 26, which has WebGPU); where there is no WebGPU — Firefox today — it falls back to WebGL and draws everything, but is not guaranteed. For real work, download the app: it is the one that brings the MCP server to your AI.",
    "aria.menu": "Menu",
    "aria.github": "Repository on GitHub",
    "alt.hero": "Dining room at dusk, rendered by the app",
    "alt.editor": "The editor with catalog, rendered floor plan and 3D view",
    "alt.video": "An AI designs a café in the editor, with the conversation in a terminal beside it",
    "aria.video": "Watch: an AI designs a café in 3D, live (1:38)",
    "demo.fig": "VIDEO — An AI designs a café from scratch, live, over MCP · 1:38",
    "alt.plan": "Rendered floor plan of the apartment",
    "alt.day": "By day",
    "alt.night": "At night",
    "aria.sections": "Sections",
    "aria.links": "Links",
    "aria.room": "Room",
    "aria.client": "AI client",
    "aria.os": "Operating system",
    "skip": "Skip to content",
    "nav.how": "How it works", "nav.gallery": "Gallery", "nav.ai": "Connect AI", "nav.faq": "FAQ", "nav.download": "Download",
    "bmc": "Buy me a coffee", "aria.bmc": "Buy me a coffee",
    "hero.eyebrow": "Open source · free · Windows, macOS and Linux",
    "hero.title1": "Design homes by talking to your AI.",
    "hero.title2": "It draws, furnishes and takes the photos. You approve.",
    "hero.download": "Download free", "hero.see": "See how it works",
    "hero.fig": "FIG. 01 — Dining room at dusk, rendered by the app",
    "works": "Works with", "works.more": "and others via MCP",
    "st.strong": "An architecture editor with an MCP server inside.",
    "st.rest": "You describe what you want; the agent uses the same tools you do — walls, doors, joinery, lighting, camera — and every change shows up on screen, one Ctrl+Z away.",
    "product.fig": "FIG. 02 — The editor: catalog, rendered floor plan and live 3D, with the AI working over MCP",
    "t1.h": "The AI edits the real file", "t1.p": "It's not an image generator. The agent edits the actual project, in centimeters, and you watch every wall appear.",
    "t2.h": "Everything is true to size", "t2.p": "A 158 × 208 cm bed is exactly that in the plan, in 3D and in collision checks.",
    "t3.h": "Runs on your computer", "t3.p": "Your project stays with you. No account, no subscription, no cloud. Renders photos even without a graphics card.",
    "cv.h1": "One conversation, one apartment.", "cv.h2": "A real 105 m² floor plan, built entirely by the AI.",
    "cv.you": "you", "cv.prompt": "Build this FOCSA floor plan at real scale, furnish it like a modern apartment and light it to code.",
    "cv.c1": "plan at scale", "cv.c2": "walls and rooms", "cv.c3": "swinging the right way", "cv.c4": "finishes", "cv.c5": "slatted wall",
    "cv.c6": "kitchen and cabinets", "cv.c8": "nothing blocked ✓", "cv.c9": "dining photo",
    "cv.fig": "FIG. 03 — Rendered floor plan exported by the app",
    "s1": "of real floor plan, traced from the image", "s2": "furniture pieces in the right place", "s3": "rooms with lighting checked against code", "s4": "MCP tools at the AI's disposal",
    "dn.h1": "3 pm sun or lamps on.", "dn.h2": "Same camera, the lights you placed.",
    "dn.dining": "Dining", "dn.kitchen": "Kitchen", "dn.suite": "Bedroom",
    "dn.fig": "FIG. 04 — Drag to compare · path tracing with the sun from the compass and fixtures in lumens",
    "dn.aria": "Compare day and night",
    "ft.h1": "Made for people who really design.", "ft.h2": "And for the AI working with them.",
    "f1.h": "A floor plan from a picture", "f1.p": "Drop in a plan image at scale and the AI finds the walls. Opens Sweet Home 3D projects too.",
    "f2.h": "Joinery a workshop can build", "f2.p": "Cabinets, slatted panels, countertops with sink and cooktop cutouts. Cut lists in CSV and DXF.",
    "f3.h": "Lighting checked against code", "f3.p": "Lux per room by photometry, against NBR ISO/CIE 8995-1. The AI adds the fixtures a room is missing.",
    "f4.h": "Ergonomics for who lives there", "f4.p": "Circulation, wheelchair turning space, the kitchen triangle — NBR 9050 and 15575, with the fix ready.",
    "f5.h": "Photos, sections and videos", "f5.p": "Rendered plans, elevations, sections, aerial views, realistic photos and walkthrough videos.",
    "f6.h": "Desktop, browser and teams", "f6.p": "The editor on your computer or in the browser, plugins in any language and several people on one project.",
    "gl.h1": "Everything here came out of the app.", "gl.h2": "No Photoshop, no render farm.", "gl.how": "How it was made →",
    "g1": "Aerial cutaway", "g2": "Slatted wall with built-in TV", "g3": "L kitchen, terrazzo", "g4": "Home office", "g5": "Bedroom at night", "g6": "Living and dining, 33 m²", "g7": "Kitchen at night", "g8": "Bathroom, Nero marble", "g9": "Bedroom with wardrobes", "g10": "Dining at 3 pm",
    "ai.h1": "Connect the AI you already use.", "ai.h2": "One command, and it can see your project.",
    "ai.s1h": "Open 3D New Era AI", "ai.s1p": "The MCP server starts with it, at <code>http://127.0.0.1:7878/mcp</code>.",
    "ai.s2h": "Run your AI's command", "ai.s2p": "Just once. The installers already do it for Claude Code and Codex.",
    "ai.s3h": "Ask", "ai.s3p": "“Draw a 4 × 5 m bedroom with a door and a window, furnish it and render a photo.”",
    "ai.s4h": "See that it worked", "ai.s4p": "In the app, the “AI” menu names the client that connected and shows the tools it is calling, as it calls them.",
    "ai.other": "DeepSeek and others", "copy": "Copy", "copied": "Copied",
    "dl.h1": "Download and start now.", "dl.h2": "Free, no account, no administrator password.",
    "dl.files": "Or download the file",
    "dl.first": "First launch of a downloaded file",
    "dl.firstp": "The apps aren't signed with a paid Apple or Microsoft certificate, so the system asks once. <b>Windows:</b> open <code>newera-gui.exe</code> and, if SmartScreen shows up, click <i>More info → Run anyway</i>. <b>macOS:</b> move the app to Applications, right-click → <i>Open</i> (macOS 15+: <i>Settings → Privacy &amp; Security → Open Anyway</i>). The command above skips these prompts.",
    "dl.notes": "release notes", "dl.license": "MIT or Apache 2.0",
    "faq.h": "FAQ",
    "q1": "Is it really free?", "a1": "Yes. It's open source (MIT or Apache 2.0), with no account and no subscription. You only pay for the AI you choose, if it's a paid one.",
    "q2": "Do I need to know how to code?", "a2": "No. Install with one command, open the app and draw with the mouse like any editor. The AI is optional — once connected, you just talk to it.",
    "q3": "Which AI works?", "a3": "Any app that speaks MCP: Claude Code, Claude Desktop, Codex, Gemini CLI, Cursor, VS Code, Windsurf, and apps like Cline and Cherry Studio that run DeepSeek, Qwen, Llama and other models.",
    "q4": "Does my project go to the cloud?", "a4": "No. The app and its MCP server run on your computer and only accept connections from the same machine. The project is your own <code>.newera</code> file.",
    "q5": "Do I need a graphics card?", "a5": "No. The 3D view uses the GPU when there is one, and photos render on the CPU — even on a headless server.",
    "q6": "Does it open my Sweet Home 3D projects?", "a6": "Yes, <code>.sh3d</code> files open with walls, rooms, furniture, lights, cameras and levels. <a href=\"/sweet-home-3d-alternative/\">How it compares with Sweet Home 3D</a>.",
    "foot.by": "Built in Rust by Leandro Ferreira.", "foot.releases": "Releases", "foot.showcase": "Showcase", "foot.issues": "Report an issue", "foot.privacy": "Privacy", "foot.terms": "Terms", "foot.support": "Support ☕", "foot.refund": "Refunds", "foot.contact": "Contact", "ai.oneclick": "Or in one click, with the app installed:",
    "foot.credits": "Reference plan: <i>Typical apartment floor plan FOCSA Building</i>, Osvaldo Valdes, CC BY-SA 4.0. Textures: ambientCG, CC0."
  };

  const ES = {
    "hero.try": "Abrir en el navegador",
    "nav.try": "Abrir en el navegador",
    "q7": "¿Funciona en el navegador?",
    "a7": "El editor completo se compila a WebAssembly y funciona en una pestaña. El soporte oficial es <b>Chrome y Edge</b> (y Safari 26, que ya tiene WebGPU); donde no hay WebGPU —hoy Firefox— pasa a WebGL y lo dibuja todo, pero no lo garantizamos. Para trabajar de verdad, descargue la aplicación: es la que trae el servidor MCP para su IA.",
    "aria.menu": "Menú",
    "aria.github": "Repositorio en GitHub",
    "alt.hero": "Comedor al atardecer, renderizado por la app",
    "alt.editor": "El editor con catálogo, plano renderizado y vista 3D",
    "alt.video": "Una IA diseña una cafetería en el editor, con la conversación en una terminal al lado",
    "aria.video": "Ver: una IA diseña una cafetería en 3D, en vivo (1:38)",
    "demo.fig": "VÍDEO — Una IA diseña una cafetería desde cero, en vivo, por MCP · 1:38",
    "alt.plan": "Plano renderizado del apartamento",
    "alt.day": "De día",
    "alt.night": "De noche",
    "aria.sections": "Secciones",
    "aria.links": "Enlaces",
    "aria.room": "Ambiente",
    "aria.client": "Cliente de IA",
    "aria.os": "Sistema operativo",
    "skip": "Saltar al contenido",
    "nav.how": "Cómo funciona",
    "nav.gallery": "Galería",
    "nav.ai": "Conectar la IA",
    "nav.faq": "Preguntas",
    "nav.download": "Descargar",
    "bmc": "Invítame un café", "aria.bmc": "Invítame un café",
    "hero.eyebrow": "Código abierto · gratis · Windows, macOS y Linux",
    "hero.title1": "Diseñe casas hablando con su IA.",
    "hero.title2": "Ella dibuja, amuebla y hace las fotos. Usted aprueba.",
    "hero.download": "Descargar gratis",
    "hero.see": "Ver cómo funciona",
    "hero.fig": "FIG. 01 — Comedor al atardecer, renderizado por la app",
    "works": "Funciona con",
    "works.more": "y otras vía MCP",
    "st.strong": "Un editor de arquitectura con un servidor MCP dentro.",
    "st.rest": "Usted describe lo que quiere; el agente usa las mismas herramientas que usted — paredes, puertas, carpintería, iluminación, cámara — y cada cambio aparece en pantalla, a un Ctrl+Z de distancia.",
    "product.fig": "FIG. 02 — El editor: catálogo, plano renderizado y 3D en vivo, con la IA trabajando por MCP",
    "t1.h": "La IA edita el archivo real",
    "t1.p": "No es un generador de imágenes. El agente edita el proyecto de verdad, en centímetros, y usted ve aparecer cada pared.",
    "t2.h": "Todo a medida real",
    "t2.p": "Una cama de 158 × 208 cm mide exactamente eso en el plano, en 3D y en la comprobación de colisiones.",
    "t3.h": "Funciona en su ordenador",
    "t3.p": "Su proyecto se queda con usted. Sin cuenta, sin suscripción, sin nube. Renderiza fotos incluso sin tarjeta gráfica.",
    "cv.h1": "Una conversación, un apartamento.",
    "cv.h2": "Un plano real de 105 m², construido entero por la IA.",
    "cv.you": "usted",
    "cv.prompt": "Levante este plano del FOCSA a escala real, amuéblelo como un apartamento moderno e ilumínelo según la norma.",
    "cv.c1": "plano a escala",
    "cv.c2": "paredes y habitaciones",
    "cv.c3": "abriendo hacia el lado correcto",
    "cv.c4": "acabados",
    "cv.c5": "panel ripado",
    "cv.c6": "cocina y armarios",
    "cv.c8": "nada bloqueado ✓",
    "cv.c9": "foto del comedor",
    "cv.fig": "FIG. 03 — Plano renderizado exportado por la app",
    "s1": "de plano real, calcado de la imagen",
    "s2": "muebles en su sitio",
    "s3": "habitaciones con la iluminación comprobada según la norma",
    "s4": "herramientas MCP a disposición de la IA",
    "dn.h1": "Sol de las 3 de la tarde o lámparas encendidas.",
    "dn.h2": "La misma cámara, las luces que usted puso.",
    "dn.dining": "Comedor",
    "dn.kitchen": "Cocina",
    "dn.suite": "Dormitorio",
    "dn.fig": "FIG. 04 — Arrastre para comparar · path tracing con el sol según la brújula y luminarias en lúmenes",
    "dn.aria": "Comparar día y noche",
    "ft.h1": "Hecho para quien proyecta de verdad.",
    "ft.h2": "Y para la IA que trabaja a su lado.",
    "f1.h": "Un plano a partir de una imagen",
    "f1.p": "Suelte una imagen del plano a escala y la IA encuentra las paredes. También abre proyectos de Sweet Home 3D.",
    "f2.h": "Carpintería que un taller puede fabricar",
    "f2.p": "Armarios, paneles ripados, encimeras con huecos para fregadero y placa. Despiece en CSV y DXF.",
    "f3.h": "Iluminación comprobada según la norma",
    "f3.p": "Lux por habitación por fotometría, según NBR ISO/CIE 8995-1. La IA añade las luminarias que faltan.",
    "f4.h": "Ergonomía para quien vive allí",
    "f4.p": "Circulación, giro de silla de ruedas, el triángulo de la cocina — NBR 9050 y 15575, con la corrección lista.",
    "f5.h": "Fotos, secciones y vídeos",
    "f5.p": "Plantas renderizadas, alzados, secciones, vistas aéreas, fotos realistas y vídeos de recorrido.",
    "f6.h": "Escritorio, navegador y equipos",
    "f6.p": "El editor en su ordenador o en el navegador, plugins en cualquier lenguaje y varias personas en un mismo proyecto.",
    "gl.h1": "Todo esto salió de la app.",
    "gl.h2": "Sin Photoshop, sin granja de render.",
    "gl.how": "Cómo se hizo →",
    "g1": "Corte aéreo",
    "g2": "Panel ripado con TV integrada",
    "g3": "Cocina en L, terrazo",
    "g4": "Despacho en casa",
    "g5": "Dormitorio de noche",
    "g6": "Salón y comedor, 33 m²",
    "g7": "Cocina de noche",
    "g8": "Baño, mármol Nero",
    "g9": "Dormitorio con armarios",
    "g10": "Comedor a las 3 de la tarde",
    "ai.h1": "Conecte la IA que ya usa.",
    "ai.h2": "Un comando y ya ve su proyecto.",
    "ai.s1h": "Abra 3D New Era AI",
    "ai.s1p": "El servidor MCP arranca con la app, en <code>http://127.0.0.1:7878/mcp</code>.",
    "ai.s2h": "Ejecute el comando de su IA",
    "ai.s2p": "Solo una vez. Los instaladores ya lo hacen para Claude Code y Codex.",
    "ai.s3h": "Pida",
    "ai.s3p": "«Dibuje un dormitorio de 4 × 5 m con una puerta y una ventana, amuéblelo y renderice una foto.»",
    "ai.s4h": "Compruebe que funcionó",
    "ai.s4p": "En la aplicación, el menú “IA” dice quién se conectó y muestra las herramientas que su IA está usando, en el momento.",
    "ai.other": "DeepSeek y otras",
    "copy": "Copiar",
    "copied": "Copiado",
    "dl.h1": "Descargue y empiece ahora.",
    "dl.h2": "Gratis, sin cuenta, sin contraseña de administrador.",
    "dl.files": "O descargue el archivo",
    "dl.first": "Primera vez que abre un archivo descargado",
    "dl.firstp": "Las apps no están firmadas con un certificado de pago de Apple ni de Microsoft, así que el sistema pregunta una vez. <b>Windows:</b> abra <code>newera-gui.exe</code> y, si aparece SmartScreen, pulse <i>Más información → Ejecutar de todas formas</i>. <b>macOS:</b> mueva la app a Aplicaciones, clic derecho → <i>Abrir</i> (macOS 15+: <i>Ajustes → Privacidad y seguridad → Abrir igualmente</i>). El comando de arriba evita estos avisos.",
    "dl.notes": "notas de la versión",
    "dl.license": "MIT o Apache 2.0",
    "faq.h": "Preguntas frecuentes",
    "q1": "¿Es realmente gratis?",
    "a1": "Sí. Es código abierto (MIT o Apache 2.0), sin cuenta y sin suscripción. Solo paga por la IA que elija, si es de pago.",
    "q2": "¿Necesito saber programar?",
    "a2": "No. Instale con un comando, abra la app y dibuje con el ratón como en cualquier editor. La IA es opcional — una vez conectada, basta con hablarle.",
    "q3": "¿Qué IA funciona?",
    "a3": "Cualquier app que hable MCP: Claude Code, Claude Desktop, Codex, Gemini CLI, Cursor, VS Code, Windsurf, y apps como Cline y Cherry Studio que ejecutan DeepSeek, Qwen, Llama y otros modelos.",
    "q4": "¿Mi proyecto va a la nube?",
    "a4": "No. La app y su servidor MCP se ejecutan en su ordenador y solo aceptan conexiones de la misma máquina. El proyecto es su propio archivo <code>.newera</code>.",
    "q5": "¿Necesito tarjeta gráfica?",
    "a5": "No. La vista 3D usa la GPU cuando la hay, y las fotos se renderizan en la CPU — incluso en un servidor sin pantalla.",
    "q6": "¿Abre mis proyectos de Sweet Home 3D?",
    "a6": "Sí, los archivos <code>.sh3d</code> se abren con paredes, habitaciones, muebles, luces, cámaras y niveles. <a href=\"/sweet-home-3d-alternative/\">Comparación con Sweet Home 3D</a> (en inglés).",
    "foot.by": "Hecho en Rust por Leandro Ferreira.",
    "foot.releases": "Versiones",
    "foot.showcase": "Galería",
    "foot.issues": "Informar de un problema",
    "foot.privacy": "Privacidad",
    "ai.oneclick": "O con un clic, con la app instalada:",
    "foot.terms": "Términos", "foot.support": "Apoyar ☕", "foot.refund": "Reembolsos", "foot.contact": "Contacto",
    "foot.credits": "Plano de referencia: <i>Typical apartment floor plan FOCSA Building</i>, Osvaldo Valdes, CC BY-SA 4.0. Texturas: ambientCG, CC0.",
  };

  const FR = {
    "hero.try": "Ouvrir dans le navigateur",
    "nav.try": "Ouvrir dans le navigateur",
    "q7": "Est-ce que ça marche dans le navigateur ?",
    "a7": "L'éditeur complet se compile en WebAssembly et tourne dans un onglet. Les navigateurs pris en charge sont <b>Chrome et Edge</b> (et Safari 26, qui a WebGPU) ; là où WebGPU manque — Firefox aujourd'hui — il bascule sur WebGL et dessine tout, sans garantie. Pour travailler vraiment, téléchargez l'application : c'est elle qui apporte le serveur MCP à votre IA.",
    "aria.menu": "Menu",
    "aria.github": "Dépôt sur GitHub",
    "alt.hero": "Salle à manger au crépuscule, rendue par l'app",
    "alt.editor": "L'éditeur avec catalogue, plan rendu et vue 3D",
    "alt.video": "Une IA conçoit un café dans l'éditeur, avec la conversation dans un terminal à côté",
    "aria.video": "Regarder : une IA conçoit un café en 3D, en direct (1:38)",
    "demo.fig": "VIDÉO — Une IA conçoit un café de zéro, en direct, par MCP · 1:38",
    "alt.plan": "Plan rendu de l'appartement",
    "alt.day": "De jour",
    "alt.night": "La nuit",
    "aria.sections": "Sections",
    "aria.links": "Liens",
    "aria.room": "Pièce",
    "aria.client": "Client IA",
    "aria.os": "Système",
    "skip": "Aller au contenu",
    "nav.how": "Comment ça marche",
    "nav.gallery": "Galerie",
    "nav.ai": "Connecter l'IA",
    "nav.faq": "Questions",
    "nav.download": "Télécharger",
    "bmc": "Offre-moi un café", "aria.bmc": "Offre-moi un café",
    "hero.eyebrow": "Open source · gratuit · Windows, macOS et Linux",
    "hero.title1": "Concevez des maisons en parlant à votre IA.",
    "hero.title2": "Elle dessine, meuble et prend les photos. Vous validez.",
    "hero.download": "Télécharger gratuitement",
    "hero.see": "Voir comment ça marche",
    "hero.fig": "FIG. 01 — Salle à manger au crépuscule, rendue par l'app",
    "works": "Fonctionne avec",
    "works.more": "et d'autres via MCP",
    "st.strong": "Un éditeur d'architecture avec un serveur MCP à l'intérieur.",
    "st.rest": "Vous décrivez ce que vous voulez ; l'agent utilise les mêmes outils que vous — murs, portes, menuiserie, éclairage, caméra — et chaque changement s'affiche à l'écran, à un Ctrl+Z près.",
    "product.fig": "FIG. 02 — L'éditeur : catalogue, plan rendu et 3D en direct, avec l'IA qui travaille par MCP",
    "t1.h": "L'IA modifie le vrai fichier",
    "t1.p": "Ce n'est pas un générateur d'images. L'agent modifie le projet lui-même, au centimètre, et vous voyez chaque mur apparaître.",
    "t2.h": "Tout à la bonne dimension",
    "t2.p": "Un lit de 158 × 208 cm fait exactement cela sur le plan, en 3D et dans les contrôles de collision.",
    "t3.h": "Tourne sur votre ordinateur",
    "t3.p": "Votre projet reste chez vous. Sans compte, sans abonnement, sans cloud. Les photos se calculent même sans carte graphique.",
    "cv.h1": "Une conversation, un appartement.",
    "cv.h2": "Un vrai plan de 105 m², construit entièrement par l'IA.",
    "cv.you": "vous",
    "cv.prompt": "Construis ce plan FOCSA à l'échelle réelle, meuble-le comme un appartement moderne et éclaire-le selon la norme.",
    "cv.c1": "plan à l'échelle",
    "cv.c2": "murs et pièces",
    "cv.c3": "ouvrant du bon côté",
    "cv.c4": "finitions",
    "cv.c5": "mur à tasseaux",
    "cv.c6": "cuisine et meubles",
    "cv.c8": "rien de bloqué ✓",
    "cv.c9": "photo de la salle à manger",
    "cv.fig": "FIG. 03 — Plan rendu, exporté par l'app",
    "s1": "de plan réel, relevé depuis l'image",
    "s2": "meubles à leur place",
    "s3": "pièces dont l'éclairage est vérifié selon la norme",
    "s4": "outils MCP à la disposition de l'IA",
    "dn.h1": "Soleil de 15 h ou lampes allumées.",
    "dn.h2": "La même caméra, les lumières que vous avez posées.",
    "dn.dining": "Salle à manger",
    "dn.kitchen": "Cuisine",
    "dn.suite": "Chambre",
    "dn.fig": "FIG. 04 — Glissez pour comparer · path tracing avec le soleil selon la boussole et des luminaires en lumens",
    "dn.aria": "Comparer le jour et la nuit",
    "ft.h1": "Fait pour celles et ceux qui conçoivent vraiment.",
    "ft.h2": "Et pour l'IA qui travaille à leurs côtés.",
    "f1.h": "Un plan à partir d'une image",
    "f1.p": "Déposez une image de plan à l'échelle et l'IA retrouve les murs. Ouvre aussi les projets Sweet Home 3D.",
    "f2.h": "De la menuiserie qu'un atelier sait fabriquer",
    "f2.p": "Meubles, panneaux à tasseaux, plans de travail avec découpes d'évier et de table de cuisson. Débits en CSV et DXF.",
    "f3.h": "Éclairage vérifié selon la norme",
    "f3.p": "Lux par pièce par photométrie, selon NBR ISO/CIE 8995-1. L'IA ajoute les luminaires qui manquent.",
    "f4.h": "Ergonomie pour ceux qui y vivent",
    "f4.p": "Circulation, aire de rotation d'un fauteuil roulant, le triangle de la cuisine — NBR 9050 et 15575, avec la correction prête.",
    "f5.h": "Photos, coupes et vidéos",
    "f5.p": "Plans rendus, élévations, coupes, vues aériennes, photos réalistes et vidéos de visite.",
    "f6.h": "Bureau, navigateur et équipes",
    "f6.p": "L'éditeur sur votre ordinateur ou dans le navigateur, des plugins dans n'importe quel langage et plusieurs personnes sur un projet.",
    "gl.h1": "Tout ici est sorti de l'app.",
    "gl.h2": "Sans Photoshop, sans ferme de rendu.",
    "gl.how": "Comment c'est fait →",
    "g1": "Coupe aérienne",
    "g2": "Mur à tasseaux avec TV intégrée",
    "g3": "Cuisine en L, terrazzo",
    "g4": "Bureau à la maison",
    "g5": "Chambre la nuit",
    "g6": "Salon et salle à manger, 33 m²",
    "g7": "Cuisine la nuit",
    "g8": "Salle de bain, marbre Nero",
    "g9": "Chambre avec dressings",
    "g10": "Salle à manger à 15 h",
    "ai.h1": "Connectez l'IA que vous utilisez déjà.",
    "ai.h2": "Une commande, et elle voit votre projet.",
    "ai.s1h": "Ouvrez 3D New Era AI",
    "ai.s1p": "Le serveur MCP démarre avec l'app, sur <code>http://127.0.0.1:7878/mcp</code>.",
    "ai.s2h": "Lancez la commande de votre IA",
    "ai.s2p": "Une seule fois. Les installateurs le font déjà pour Claude Code et Codex.",
    "ai.s3h": "Demandez",
    "ai.s3p": "« Dessine une chambre de 4 × 5 m avec une porte et une fenêtre, meuble-la et fais-en une photo. »",
    "ai.s4h": "Vérifiez que ça marche",
    "ai.s4p": "Dans l'application, le menu « IA » nomme le client connecté et montre les outils que votre IA appelle, au moment où elle les appelle.",
    "ai.other": "DeepSeek et autres",
    "copy": "Copier",
    "copied": "Copié",
    "dl.h1": "Téléchargez et commencez tout de suite.",
    "dl.h2": "Gratuit, sans compte, sans mot de passe administrateur.",
    "dl.files": "Ou téléchargez le fichier",
    "dl.first": "Première ouverture d'un fichier téléchargé",
    "dl.firstp": "Les applications ne sont pas signées avec un certificat payant Apple ou Microsoft : le système pose donc la question une fois. <b>Windows :</b> ouvrez <code>newera-gui.exe</code> et, si SmartScreen apparaît, cliquez sur <i>Informations complémentaires → Exécuter quand même</i>. <b>macOS :</b> déplacez l'app dans Applications, clic droit → <i>Ouvrir</i> (macOS 15+ : <i>Réglages → Confidentialité et sécurité → Ouvrir quand même</i>). La commande ci-dessus évite ces messages.",
    "dl.notes": "notes de version",
    "dl.license": "MIT ou Apache 2.0",
    "faq.h": "Questions fréquentes",
    "q1": "C'est vraiment gratuit ?",
    "a1": "Oui. C'est open source (MIT ou Apache 2.0), sans compte et sans abonnement. Vous ne payez que l'IA que vous choisissez, si elle est payante.",
    "q2": "Faut-il savoir programmer ?",
    "a2": "Non. Installez avec une commande, ouvrez l'app et dessinez à la souris comme dans n'importe quel éditeur. L'IA est optionnelle — une fois connectée, il suffit de lui parler.",
    "q3": "Quelle IA fonctionne ?",
    "a3": "Toute application qui parle MCP : Claude Code, Claude Desktop, Codex, Gemini CLI, Cursor, VS Code, Windsurf, et des applications comme Cline et Cherry Studio qui font tourner DeepSeek, Qwen, Llama et d'autres modèles.",
    "q4": "Mon projet part-il dans le cloud ?",
    "a4": "Non. L'app et son serveur MCP tournent sur votre ordinateur et n'acceptent que les connexions de la même machine. Le projet est votre propre fichier <code>.newera</code>.",
    "q5": "Faut-il une carte graphique ?",
    "a5": "Non. La vue 3D utilise le GPU quand il y en a un, et les photos se calculent sur le CPU — même sur un serveur sans écran.",
    "q6": "Ouvre-t-il mes projets Sweet Home 3D ?",
    "a6": "Oui, les fichiers <code>.sh3d</code> s'ouvrent avec murs, pièces, meubles, lumières, caméras et niveaux. <a href=\"/sweet-home-3d-alternative/\">Comparaison avec Sweet Home 3D</a> (en anglais).",
    "foot.by": "Écrit en Rust par Leandro Ferreira.",
    "foot.releases": "Versions",
    "foot.showcase": "Galerie",
    "foot.issues": "Signaler un problème",
    "foot.privacy": "Confidentialité",
    "ai.oneclick": "Ou en un clic, l’app installée :",
    "foot.terms": "Conditions", "foot.support": "Soutenir ☕", "foot.refund": "Remboursement", "foot.contact": "Contact",
    "foot.credits": "Plan de référence : <i>Typical apartment floor plan FOCSA Building</i>, Osvaldo Valdes, CC BY-SA 4.0. Textures : ambientCG, CC0.",
  };

  const T = {
    pt: {
      winLead: "Abra o <b>PowerShell</b> (menu Iniciar → digite “PowerShell”) e cole:",
      winAfter: "Instala em segundos, cria atalho no Menu Iniciar e na Área de Trabalho e aparece em Configurações → Aplicativos para desinstalar.",
      macLead: "Abra o <b>Terminal</b> (⌘ + espaço → “Terminal”) e cole:",
      macAfter: "Baixa a versão certa para Apple Silicon ou Intel, coloca o app no Launchpad e não pede senha. Rode de novo para atualizar.",
      linLead: "Baixe, descompacte e abra:",
      linAfter: "Precisa das bibliotecas de janela do sistema (já presentes na maioria das distribuições com desktop).",
      dlWin: "Baixar para Windows", dlMac: "Baixar para Mac", dlLinux: "Baixar para Linux", dlAny: "Baixar grátis",
      where: { claude: "No terminal:", codex: "No terminal:", gemini: "No terminal:", vscode: "No terminal:", cursor: "Arquivo ~/.cursor/mcp.json", desktop: "Configurações → Desenvolvedor → Editar configuração", other: "Nas configurações de MCP do app (Cline, Roo Code, Cherry Studio, LM Studio…)" },
      note: {
        claude: "Os instaladores já registram sozinhos se o Claude Code estiver instalado.",
        codex: "Os instaladores já registram sozinhos se o Codex estiver instalado.",
        gemini: "Depois abra o Gemini CLI e peça o projeto.",
        vscode: "Use no modo agente do Copilot.",
        cursor: "Reinicie o Cursor depois de salvar.",
        desktop: "Precisa do Node.js instalado para a ponte mcp-remote. Reinicie o Claude Desktop.",
        other: "O modelo pode ser DeepSeek, Qwen, Llama ou outro: o que importa é o app ter suporte a MCP. No opencode, use o bloco acima em opencode.json."
      }
    },
    es: {
      winLead: "Abra <b>PowerShell</b> (menú Inicio → escriba «PowerShell») y pegue:",
      winAfter: "Se instala en segundos, crea accesos directos en el menú Inicio y el Escritorio, y aparece en Configuración → Aplicaciones para desinstalar.",
      macLead: "Abra el <b>Terminal</b> (⌘ + espacio → «Terminal») y pegue:",
      macAfter: "Descarga la versión correcta para Apple Silicon o Intel, deja la app en el Launchpad y no pide contraseña. Ejecútelo otra vez para actualizar.",
      linLead: "Descargue, descomprima y abra:",
      linAfter: "Necesita las bibliotecas de ventanas del sistema (ya presentes en casi toda distribución con escritorio).",
      dlWin: "Descargar para Windows", dlMac: "Descargar para Mac", dlLinux: "Descargar para Linux", dlAny: "Descargar gratis",
      where: { claude: "En el terminal:", codex: "En el terminal:", gemini: "En el terminal:", vscode: "En el terminal:", cursor: "Archivo ~/.cursor/mcp.json", desktop: "Configuración → Desarrollador → Editar configuración", other: "En los ajustes de MCP de la app (Cline, Roo Code, Cherry Studio, LM Studio…)" },
      note: {
        claude: "Los instaladores lo registran solos si Claude Code está instalado.",
        codex: "Los instaladores lo registran solos si Codex está instalado.",
        gemini: "Después abra Gemini CLI y pídale el proyecto.",
        vscode: "Úselo en el modo agente de Copilot.",
        cursor: "Reinicie Cursor después de guardar.",
        desktop: "Necesita Node.js para el puente mcp-remote. Reinicie Claude Desktop.",
        other: "El modelo puede ser DeepSeek, Qwen, Llama u otro: lo que importa es que la app hable MCP. En opencode, use el bloque de arriba en opencode.json."
      }
    },
    fr: {
      winLead: "Ouvrez <b>PowerShell</b> (menu Démarrer → tapez « PowerShell ») et collez :",
      winAfter: "L'installation prend quelques secondes, ajoute des raccourcis au menu Démarrer et au Bureau, et apparaît dans Paramètres → Applications pour désinstaller.",
      macLead: "Ouvrez le <b>Terminal</b> (⌘ + espace → « Terminal ») et collez :",
      macAfter: "Télécharge la bonne version pour Apple Silicon ou Intel, pose l'app dans le Launchpad et ne demande aucun mot de passe. Relancez-le pour mettre à jour.",
      linLead: "Téléchargez, décompressez et lancez :",
      linAfter: "Nécessite les bibliothèques de fenêtrage du système (déjà présentes sur la plupart des distributions avec bureau).",
      dlWin: "Télécharger pour Windows", dlMac: "Télécharger pour Mac", dlLinux: "Télécharger pour Linux", dlAny: "Télécharger gratuitement",
      where: { claude: "Dans un terminal :", codex: "Dans un terminal :", gemini: "Dans un terminal :", vscode: "Dans un terminal :", cursor: "Fichier ~/.cursor/mcp.json", desktop: "Réglages → Développeur → Modifier la configuration", other: "Dans les réglages MCP de l'app (Cline, Roo Code, Cherry Studio, LM Studio…)" },
      note: {
        claude: "Les installateurs l'enregistrent tout seuls si Claude Code est installé.",
        codex: "Les installateurs l'enregistrent tout seuls si Codex est installé.",
        gemini: "Ouvrez ensuite Gemini CLI et demandez-lui votre projet.",
        vscode: "À utiliser dans le mode agent de Copilot.",
        cursor: "Redémarrez Cursor après avoir enregistré.",
        desktop: "Nécessite Node.js pour le pont mcp-remote. Redémarrez Claude Desktop.",
        other: "Le modèle peut être DeepSeek, Qwen, Llama ou un autre : ce qui compte, c'est que l'app parle MCP. Dans opencode, mettez le bloc ci-dessus dans opencode.json."
      }
    },
    en: {
      winLead: "Open <b>PowerShell</b> (Start menu → type “PowerShell”) and paste:",
      winAfter: "Installs in seconds, adds Start menu and desktop shortcuts, and shows up in Settings → Apps to uninstall.",
      macLead: "Open <b>Terminal</b> (⌘ + space → “Terminal”) and paste:",
      macAfter: "Picks the right build for Apple Silicon or Intel, puts the app in Launchpad and never asks for a password. Run it again to update.",
      linLead: "Download, extract and run:",
      linAfter: "Needs the system's windowing libraries (already there on most desktop distributions).",
      dlWin: "Download for Windows", dlMac: "Download for Mac", dlLinux: "Download for Linux", dlAny: "Download free",
      where: { claude: "In a terminal:", codex: "In a terminal:", gemini: "In a terminal:", vscode: "In a terminal:", cursor: "File ~/.cursor/mcp.json", desktop: "Settings → Developer → Edit Config", other: "In the app's MCP settings (Cline, Roo Code, Cherry Studio, LM Studio…)" },
      note: {
        claude: "The installers register it for you when Claude Code is installed.",
        codex: "The installers register it for you when Codex is installed.",
        gemini: "Then open Gemini CLI and ask for your project.",
        vscode: "Use it in Copilot's agent mode.",
        cursor: "Restart Cursor after saving.",
        desktop: "Needs Node.js for the mcp-remote bridge. Restart Claude Desktop.",
        other: "The model can be DeepSeek, Qwen, Llama or anything else: what matters is that the app speaks MCP. In opencode, put the block above in opencode.json."
      }
    }
  };

  const CODE = {
    claude: `claude mcp add --transport http newera ${MCP}`,
    codex: `codex mcp add newera --url ${MCP}`,
    gemini: `gemini mcp add --transport http newera ${MCP}`,
    vscode: `code --add-mcp '{"name":"newera","type":"http","url":"${MCP}"}'`,
    cursor: `{\n  "mcpServers": {\n    "newera": { "url": "${MCP}" }\n  }\n}`,
    desktop: `{\n  "mcpServers": {\n    "newera": {\n      "command": "npx",\n      "args": ["-y", "mcp-remote", "${MCP}"]\n    }\n  }\n}`,
    other: `{\n  "mcp": {\n    "newera": { "type": "remote", "url": "${MCP}" }\n  }\n}`
  };

  const OS_CODE = {
    windows: `irm ${RAW}/install-windows.ps1 | iex`,
    mac: `curl -fsSL ${RAW}/install-macos.sh | bash`,
    linux: `curl -LO https://github.com/${REPO}/releases/latest/download/newera-linux-x64.tar.gz\ntar xzf newera-linux-x64.tar.gz && ./newera/newera`
  };

  // The tab title and the description a link preview shows. The page ships
  // in English so a shared link reads to the whole world; the visitor's own
  // language takes over as soon as the script runs.
  const HEAD = {
    pt: {
      title: "3D New Era AI — projete casas conversando com a sua IA",
      description: "Editor de arquitetura e interiores de código aberto com servidor MCP embutido. Sua IA desenha, mobilia, ilumina pela NBR e fotografa — ao vivo na sua tela. Windows, macOS e Linux."
    },
    en: {
      title: "3D New Era AI — design homes by talking to your AI",
      description: "Open-source architecture and interiors editor with an MCP server inside. Your AI draws, furnishes, lights to code and renders the photos — live on your screen. Windows, macOS and Linux."
    },
    es: {
      title: "3D New Era AI — diseñe casas hablando con su IA",
      description: "Editor de arquitectura e interiores de código abierto con servidor MCP dentro. Su IA dibuja, amuebla, ilumina según la norma y fotografía — en vivo en su pantalla. Windows, macOS y Linux."
    },
    fr: {
      title: "3D New Era AI — concevez des maisons en parlant à votre IA",
      description: "Éditeur d'architecture et d'intérieurs open source avec un serveur MCP intégré. Votre IA dessine, meuble, éclaire selon la norme et fait les photos — en direct sur votre écran. Windows, macOS et Linux."
    }
  };

  const $ = (s, root = document) => root.querySelector(s);
  const $$ = (s, root = document) => Array.from(root.querySelectorAll(s));

  // ---- Language ----
  // The page is written in Portuguese in the markup; the other three come
  // from the tables above.
  const PT = {};
  $$("[data-i18n]").forEach((el) => { PT[el.dataset.i18n] = el.innerHTML; });
  $$("[data-i18n-alt]").forEach((el) => { PT[el.dataset.i18nAlt] ??= el.getAttribute("alt"); });
  $$("[data-i18n-label]").forEach((el) => {
    PT[el.dataset.i18nLabel] ??= el.getAttribute("aria-label");
  });
  const DICT = { pt: PT, en: EN, es: ES, fr: FR };
  const TAG = { pt: "pt-BR", en: "en", es: "es", fr: "fr" };
  const LANGS = Object.keys(DICT);
  let lang = "pt";
  let tab = "claude";
  let os = detectOS();

  function store(key, value) { try { localStorage.setItem(key, value); } catch (_) { /* private mode */ } }
  function read(key) { try { return localStorage.getItem(key); } catch (_) { return null; } }

  /// The two letters of a tag like `pt-BR`, `es-419` or `fr`, when the page
  /// speaks it.
  function normalize(tag) {
    const base = String(tag || "").toLowerCase().slice(0, 2);
    return LANGS.includes(base) ? base : null;
  }

  // What the browser asks for, in the order it prefers. English for the rest
  // of the world, which is what a visitor from anywhere reads.
  function fromBrowser() {
    const asked = (navigator.languages && navigator.languages.length)
      ? navigator.languages
      : [navigator.language];
    for (const tag of asked) {
      const known = normalize(tag);
      if (known) return known;
    }
    return "en";
  }

  function setLang(next) {
    lang = normalize(next) || "en";
    const dict = DICT[lang];
    $$("[data-i18n]").forEach((el) => {
      const value = dict[el.dataset.i18n];
      if (value !== undefined) el.innerHTML = value;
    });
    // What a screen reader and a search engine read: the picture
    // descriptions and the names of the groups of controls.
    $$("[data-i18n-alt]").forEach((el) => {
      const value = dict[el.dataset.i18nAlt];
      if (value !== undefined) el.setAttribute("alt", value);
    });
    $$("[data-i18n-label]").forEach((el) => {
      const value = dict[el.dataset.i18nLabel];
      if (value !== undefined) el.setAttribute("aria-label", value);
    });
    document.documentElement.lang = TAG[lang];
    $$(".lang button").forEach((b) => b.setAttribute("aria-pressed", String(b.dataset.lang === lang)));
    const range = $(".compare__range");
    if (range) range.setAttribute("aria-label", dict["dn.aria"] || EN["dn.aria"]);
    const head = HEAD[lang];
    document.title = head.title;
    const description = $('meta[name="description"]');
    if (description) description.setAttribute("content", head.description);
    renderTab();
    renderOS();
    store("newera-lang", lang);
  }

  $$(".lang button").forEach((b) => b.addEventListener("click", () => setLang(b.dataset.lang)));

  // ---- The demo video: YouTube loads only for whoever presses play ----
  $$("[data-youtube]").forEach((link) => link.addEventListener("click", (event) => {
    if (event.metaKey || event.ctrlKey || event.shiftKey) return;
    event.preventDefault();
    const frame = document.createElement("iframe");
    frame.src = `https://www.youtube-nocookie.com/embed/${link.dataset.youtube}?autoplay=1&rel=0&modestbranding=1&playsinline=1`;
    frame.title = link.getAttribute("aria-label");
    frame.allow = "autoplay; encrypted-media; picture-in-picture; fullscreen";
    frame.allowFullscreen = true;
    link.replaceWith(frame);
    // The link had the focus; a keyboard keeps it on the player that took its place.
    frame.focus();
  }));

  // ---- The menu a narrow window gets ----
  const toggle = $(".nav__toggle");
  const menu = $(".nav__menu");
  function openMenu(open) {
    if (!toggle || !menu) return;
    toggle.setAttribute("aria-expanded", String(open));
    menu.hidden = !open;
  }
  if (toggle && menu) {
    toggle.addEventListener("click", () => {
      openMenu(toggle.getAttribute("aria-expanded") !== "true");
    });
    menu.addEventListener("click", (event) => {
      if (event.target.closest("a")) openMenu(false);
    });
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape") openMenu(false);
    });
    // A window wide enough for the row of links has no use for the panel.
    addEventListener("resize", () => { if (innerWidth > 1000) openMenu(false); });
  }

  // ---- Stars, when GitHub feels like answering ----
  (async () => {
    const slots = $$(".stars__n");
    if (!slots.length) return;
    const show = (count) => {
      const text = count >= 1000 ? `${(count / 1000).toFixed(1)}k` : String(count);
      for (const slot of slots) {
        slot.textContent = text;
        slot.hidden = false;
      }
    };
    const cached = read("newera-stars");
    if (cached) {
      const [count, at] = cached.split(":").map(Number);
      if (count > 0) show(count);
      // A day old is old enough to ask again.
      if (Date.now() - at < 864e5) return;
    }
    try {
      const response = await fetch(`https://api.github.com/repos/${REPO}`, {
        headers: { Accept: "application/vnd.github+json" }
      });
      if (!response.ok) return;
      const { stargazers_count: count } = await response.json();
      if (typeof count === "number") {
        show(count);
        store("newera-stars", `${count}:${Date.now()}`);
      }
    } catch (_) { /* offline, rate limited, blocked: the mark stands alone */ }
  })();

  // ---- OS detection ----
  function detectOS() {
    const platform = (navigator.userAgentData && navigator.userAgentData.platform) || navigator.platform || "";
    const ua = navigator.userAgent || "";
    if (/win/i.test(platform) || /Windows/i.test(ua)) return "windows";
    if (/mac/i.test(platform) || /Mac OS X/i.test(ua)) return /iPhone|iPad/i.test(ua) ? "mac" : "mac";
    if (/linux/i.test(platform) && !/Android/i.test(ua)) return "linux";
    return "windows";
  }
  const detected = (() => {
    const ua = navigator.userAgent || "";
    if (/Android|iPhone|iPad/i.test(ua)) return null;
    return detectOS();
  })();

  function renderOS() {
    const t = T[lang];
    $$(".os button").forEach((b) => b.setAttribute("aria-selected", String(b.dataset.os === os)));
    const lead = { windows: t.winLead, mac: t.macLead, linux: t.linLead }[os];
    const after = { windows: t.winAfter, mac: t.macAfter, linux: t.linAfter }[os];
    $("[data-os-lead]").innerHTML = lead;
    $("[data-os-after]").textContent = after;
    $("[data-os-code]").textContent = OS_CODE[os];
    const label = $("[data-download-label]");
    if (label) label.textContent = detected ? { windows: t.dlWin, mac: t.dlMac, linux: t.dlLinux }[detected] : t.dlAny;
  }
  $$(".os button").forEach((b) => b.addEventListener("click", () => { os = b.dataset.os; renderOS(); }));
  $$(".os button").forEach((b, i, all) => b.addEventListener("keydown", (e) => arrowNav(e, all, i)));

  // ---- AI client tabs ----
  function renderTab() {
    const t = T[lang];
    $$(".tabs__list button").forEach((b) => b.setAttribute("aria-selected", String(b.dataset.tab === tab)));
    $("[data-where]").textContent = t.where[tab];
    $("[data-code]").textContent = CODE[tab];
    $("[data-note]").textContent = t.note[tab];
  }
  $$(".tabs__list button").forEach((b) => b.addEventListener("click", () => { tab = b.dataset.tab; renderTab(); }));
  $$(".tabs__list button").forEach((b, i, all) => b.addEventListener("keydown", (e) => arrowNav(e, all, i)));

  function arrowNav(e, all, i) {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    const next = all[(i + (e.key === "ArrowRight" ? 1 : all.length - 1)) % all.length];
    next.focus();
    next.click();
  }

  // ---- Copy ----
  function copy(button, text) {
    const done = () => {
      button.textContent = lang === "en" ? EN.copied : "Copiado";
      button.classList.add("is-done");
      setTimeout(() => { button.textContent = lang === "en" ? EN.copy : PT.copy; button.classList.remove("is-done"); }, 1800);
    };
    if (navigator.clipboard && window.isSecureContext) {
      navigator.clipboard.writeText(text).then(done, () => fallback(text, done));
    } else fallback(text, done);
  }
  function fallback(text, done) {
    const area = document.createElement("textarea");
    area.value = text; area.setAttribute("readonly", ""); area.style.position = "fixed"; area.style.opacity = "0";
    document.body.appendChild(area); area.select();
    try { document.execCommand("copy"); done(); } catch (_) { /* nothing to do */ }
    area.remove();
  }
  $("[data-copy]").addEventListener("click", (e) => copy(e.currentTarget, CODE[tab]));
  $("[data-copy-os]").addEventListener("click", (e) => copy(e.currentTarget, OS_CODE[os]));

  // ---- What people leave with ----
  //
  // The page counts a download the same way the installers count an install, so
  // the two halves of the same visit can be read together: how many took the
  // one-line command, how many took a file, and for which system. Nothing here
  // is required for the page to work — no gtag, no event, no difference.
  const counted = (name, params) => {
    if (typeof window.gtag === "function") window.gtag("event", name, params);
  };
  // Which system a release file belongs to, by its name.
  const platformOf = (href) => {
    if (/windows/.test(href)) return "windows";
    if (/macos-apple-silicon/.test(href)) return "macos-aarch64";
    if (/macos-intel/.test(href)) return "macos-x86_64";
    if (/linux/.test(href)) return "linux";
    if (/\.mcpb$/.test(href)) return "mcpb";
    return "other";
  };
  $$('a[href*="/releases/latest/download/"]').forEach((link) =>
    link.addEventListener("click", () =>
      counted("download", { channel: "file", platform: platformOf(link.getAttribute("href") || "") })
    )
  );
  // Copying the install command is the other way out of this page, and the one
  // the installers then report from.
  $("[data-copy-os]").addEventListener("click", () => counted("download", { channel: "command", platform: os }));

  // ---- Day / night ----
  const SCENES = {
    jantar: ["02-dining-living", "09-dining-night", "19:30"],
    cozinha: ["04-kitchen", "10-kitchen-night", "19:30"],
    suite: ["07-master-bedroom", "11-master-bedroom-night", "20:00"]
  };
  const compare = $(".compare");
  const range = $(".compare__range");
  const setPos = (v) => compare.style.setProperty("--pos", `${v}%`);
  range.addEventListener("input", () => setPos(range.value));
  $$(".chips button").forEach((b) => b.addEventListener("click", () => {
    const [day, night, hour] = SCENES[b.dataset.scene];
    $$(".chips button").forEach((c) => c.setAttribute("aria-selected", String(c === b)));
    $("[data-day]").src = `images/showcase/${day}.jpg`;
    $("[data-night]").src = `images/showcase/${night}.jpg`;
    $(".compare__tag--r").textContent = hour;
  }));
  // A gentle hint of what the slider does, once, when it first comes into view.
  const hint = () => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
    let t0 = null;
    const step = (t) => {
      if (t0 === null) t0 = t;
      const p = Math.min((t - t0) / 1600, 1);
      const v = 50 + Math.sin(p * Math.PI * 2) * 18 * (1 - p);
      setPos(v.toFixed(2)); range.value = v;
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  };

  // ---- Reveal on scroll ----
  const reveal = $$("[data-reveal]");
  if ("IntersectionObserver" in window) {
    const io = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) return;
        const el = entry.target;
        el.classList.add("is-in");
        if (el.classList.contains("term")) {
          $$(".term__calls li", el).forEach((li, i) => { li.style.transitionDelay = `${250 + i * 170}ms`; });
        }
        if (el === compare) hint();
        io.unobserve(el);
      });
    }, { threshold: 0.25 });
    reveal.forEach((el) => io.observe(el));
    io.observe(compare);
    // Never leave content hidden if the observer doesn't fire (old browsers, print, crawlers).
    setTimeout(() => reveal.forEach((el) => el.classList.add("is-in")), 6000);
  } else {
    reveal.forEach((el) => el.classList.add("is-in"));
  }

  // ---- Product Hunt badge ----
  (() => {
    const slot = $("#ph-badge");
    if (!slot || !PRODUCT_HUNT.post) return;
    const paint = () => {
      const dark = document.documentElement.dataset.theme !== "light";
      const src = `https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=${PRODUCT_HUNT.post}&theme=${dark ? "dark" : "light"}`;
      slot.innerHTML = `<img src="${src}" width="250" height="54" alt="3D New Era AI on Product Hunt" loading="lazy">`;
    };
    slot.href = `https://www.producthunt.com/posts/${PRODUCT_HUNT.slug}?utm_source=badge-featured&utm_medium=badge`;
    slot.hidden = false;
    paint();
    new MutationObserver(paint).observe(document.documentElement, { attributeFilter: ["data-theme"] });
  })();

  // ---- Nav border on scroll ----
  const nav = $(".nav");
  const onScroll = () => nav.classList.toggle("is-scrolled", window.scrollY > 8);
  window.addEventListener("scroll", onScroll, { passive: true });
  onScroll();

  // ---- Latest version ----
  fetch(`https://api.github.com/repos/${REPO}/releases/latest`, { headers: { Accept: "application/vnd.github+json" } })
    .then((r) => (r.ok ? r.json() : null))
    .then((release) => { if (release && release.tag_name) $$("[data-version]").forEach((el) => { el.textContent = release.tag_name; }); })
    .catch(() => {});

  // ---- Start ----
  const params = new URLSearchParams(location.search);
  if (params.get("theme") === "light" || params.get("theme") === "dark") document.documentElement.dataset.theme = params.get("theme");
  // What the address says wins, then what this visitor chose before, then
  // what their browser asks for.
  const initial = normalize(params.get("lang")) || normalize(read("newera-lang")) || fromBrowser();
  if (detected) os = detected;
  setLang(initial);
})();
