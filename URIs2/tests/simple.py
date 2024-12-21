#!/usr/bin/env python
import cgi
import cgitb
import html
from datetime import datetime

# Activer le débogage CGI
cgitb.enable()

# Entête HTTP
print("Content-Type: text/html\n")

# Début du document HTML
print("""<!DOCTYPE html>
<html>
<head>
    <title>Mon Script CGI Python</title>
    <meta charset="utf-8">
</head>
<body>
    <h1>Démo CGI Python</h1>""")

# Récupérer les données du formulaire
form = cgi.FieldStorage()

# Traiter les données si elles existent
if form.getvalue("nom"):
    nom = html.escape(form.getvalue("nom"))
    print(f"<p>Bonjour, {nom}!</p>")

# Afficher l'heure actuelle
print(f"<p>Il est actuellement: {datetime.now().strftime('%H:%M:%S')}</p>")

# Formulaire HTML
print("""
    <form method="post" action="">
        <label for="nom">Votre nom:</label>
        <input type="text" id="nom" name="nom">
        <input type="submit" value="Envoyer">
    </form>
""")

# Fin du document HTML
print("""</body>
</html>""")