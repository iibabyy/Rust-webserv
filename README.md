# **webServer**

Un serveur web en asynchrone, rapide et simple basé sur Rust, capable de servir des fichiers statiques, exécuter des scripts CGI, et rediriger les requêtes avec une configuration flexible.

---

## **Configuration**  

La configuration du serveur repose sur des blocs de type **`server`** et **`location`**, similaires à ceux de NGINX. Voici quelques points clés de la configuration :  

- **Ports multiples** : Chaque bloc `server` peut écouter sur un port différent.  
- **Support CGI** : Exécution des scripts `.py`, `.sh`, et `.php` via des interpréteurs définis.  
- **Redirections** : Gestion facile des redirections permanentes ou temporaires.
- **Limitations personnalisées** : Définir des tailles de body, méthodes autorisées ou règles spécifiques par chemin.  

Il vous est possible de la modifier via le fichier conf.conf
---

## **Lancer le serveur**  

1. Clone le projet et lance le :  
   ```bash
   git clone https://github.com/iibabyy/Rust-webserv.git
   cd Rust-webserv
   make unzip
   make
