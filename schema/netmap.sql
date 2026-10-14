/*
 Navicat MySQL Data Transfer

 Source Server         : siriusnetmap
 Source Server Type    : MySQL
 Source Server Version : 50647
 Source Host           : 192.168.10.24:3306
 Source Schema         : netmap

 Target Server Type    : MySQL
 Target Server Version : 50647
 File Encoding         : 65001

 Date: 14/10/2026 10:53:25
*/

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

-- ----------------------------
-- Table structure for cell_db
-- ----------------------------
DROP TABLE IF EXISTS `cell_db`;
CREATE TABLE `cell_db` (
  `radio` varchar(255) DEFAULT NULL,
  `mcc` varchar(255) NOT NULL,
  `net` varchar(255) NOT NULL,
  `area` varchar(255) DEFAULT NULL,
  `cell` varchar(255) NOT NULL,
  `unit` varchar(255) DEFAULT NULL,
  `lon` varchar(255) DEFAULT NULL,
  `lat` varchar(255) DEFAULT NULL,
  `range` varchar(255) DEFAULT NULL,
  `samples` varchar(255) DEFAULT NULL,
  `changeable` varchar(255) DEFAULT NULL,
  `created` varchar(255) DEFAULT NULL,
  `updated` varchar(255) DEFAULT NULL,
  `averageSignal` varchar(255) DEFAULT NULL,
  `address` varchar(255) DEFAULT NULL,
  KEY `mcc` (`mcc`,`net`,`cell`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for cell_mnc
-- ----------------------------
DROP TABLE IF EXISTS `cell_mnc`;
CREATE TABLE `cell_mnc` (
  `value` int(3) NOT NULL,
  PRIMARY KEY (`value`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for comuni
-- ----------------------------
DROP TABLE IF EXISTS `comuni`;
CREATE TABLE `comuni` (
  `comune` varchar(255) NOT NULL,
  `provincia` varchar(255) NOT NULL,
  `table_name` varchar(255) NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for copertura_fttcab
-- ----------------------------
DROP TABLE IF EXISTS `copertura_fttcab`;
CREATE TABLE `copertura_fttcab` (
  `IDBRE sede OLT` varchar(255) DEFAULT NULL,
  `Cod. GAT sede OLT` varchar(255) DEFAULT NULL,
  `CLLI della centrale sede OLT` varchar(255) DEFAULT NULL,
  `Nome Sede OLT` varchar(255) DEFAULT NULL,
  `Regione OLT` varchar(255) DEFAULT NULL,
  `Distretto OLT` varchar(255) DEFAULT NULL,
  `Comune OLT` varchar(255) DEFAULT NULL,
  `ISTAT Comune OLT` varchar(255) DEFAULT NULL,
  `IDBRE Sede FEEDER` varchar(255) DEFAULT NULL,
  `CLLI Sede FEEDER` varchar(255) DEFAULT NULL,
  `Descrizione sede FEEDER` varchar(255) DEFAULT NULL,
  `Macro Area` varchar(255) DEFAULT NULL,
  `CLLI ONU` varchar(255) DEFAULT NULL,
  `ID Sede ONU` varchar(255) DEFAULT NULL,
  `Tipo ONU` varchar(255) DEFAULT NULL,
  `Stato di variazione` varchar(255) DEFAULT NULL,
  `Massima velocità VDSL2` varchar(255) DEFAULT NULL,
  `Note` varchar(255) DEFAULT NULL,
  `Stato ONU` varchar(255) DEFAULT NULL,
  `Previsione di ampliamento` varchar(255) DEFAULT NULL,
  `Semaforo giallo` varchar(255) DEFAULT NULL,
  `Risorsa generante il semaforo` varchar(255) DEFAULT NULL,
  `% di occupazione` varchar(255) DEFAULT NULL,
  `Data comunic. TI dell'apertura sede ai servizi FTTCab` varchar(255) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for ftth_fw
-- ----------------------------
DROP TABLE IF EXISTS `ftth_fw`;
CREATE TABLE `ftth_fw` (
  `id_record` int(20) DEFAULT NULL,
  `id_egon_civico` varchar(255) DEFAULT NULL,
  `particella_top_egon` varchar(255) DEFAULT NULL,
  `nome_via_egon` varchar(255) DEFAULT NULL,
  `civico_top_egon` varchar(255) DEFAULT NULL,
  `id_egon_via` varchar(255) DEFAULT NULL,
  `cap_comune` varchar(255) DEFAULT NULL,
  `comune_egon` varchar(255) DEFAULT NULL,
  `provincia_egon` varchar(255) DEFAULT NULL,
  `sigla_provincia_egon` varchar(255) DEFAULT NULL,
  `regione` varchar(255) DEFAULT NULL,
  `stato_vendibilita_gpon` varchar(255) DEFAULT NULL,
  `stato_tecnico_gpon` varchar(255) DEFAULT NULL,
  `previsione_ampliamento_gpon` varchar(255) DEFAULT NULL,
  `flag_opa` varchar(255) DEFAULT NULL,
  KEY `id_egon_civico` (`id_egon_civico`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for ftts_fw
-- ----------------------------
DROP TABLE IF EXISTS `ftts_fw`;
CREATE TABLE `ftts_fw` (
  `id_record` int(20) NOT NULL,
  `id_egon_civico` varchar(255) DEFAULT NULL,
  `particella_top_egon` varchar(255) DEFAULT NULL,
  `nome_via_egon` varchar(255) DEFAULT NULL,
  `civico_top_egon` varchar(255) DEFAULT NULL,
  `id_egon_via` varchar(255) DEFAULT NULL,
  `cap_comune` varchar(255) DEFAULT NULL,
  `comune_egon` varchar(255) DEFAULT NULL,
  `provincia_egon` varchar(255) DEFAULT NULL,
  `sigla_provincia_egon` varchar(255) DEFAULT NULL,
  `regione` varchar(255) DEFAULT NULL,
  `stato_vendibilita_ftts` varchar(255) DEFAULT NULL,
  `stato_tecnico_ftts` varchar(255) DEFAULT NULL,
  `previsione_ampliamento_ftts` varchar(255) DEFAULT NULL,
  `flag_opa` varchar(255) DEFAULT NULL,
  `id_sgu_ftts` varchar(255) DEFAULT NULL,
  `nome_sgu_ftts` varchar(255) DEFAULT NULL,
  `id_cabinet` varchar(255) DEFAULT NULL,
  `tipo_ftts` varchar(255) DEFAULT NULL,
  `distanza_civico_cabinet` varchar(255) DEFAULT NULL,
  `categoria` varchar(255) DEFAULT NULL,
  `velocita_down_ftts` varchar(255) DEFAULT NULL,
  `velocita_up_ftts` varchar(255) DEFAULT NULL,
  KEY `id_egon_civico` (`id_egon_civico`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for gpon_fw
-- ----------------------------
DROP TABLE IF EXISTS `gpon_fw`;
CREATE TABLE `gpon_fw` (
  `id_egon_civico` varchar(255) DEFAULT NULL,
  `id_egon_strada` varchar(255) DEFAULT NULL,
  `regione` varchar(255) DEFAULT NULL,
  `provincia` varchar(255) DEFAULT NULL,
  `citta` varchar(255) DEFAULT NULL,
  `particella_toponomastica` varchar(255) DEFAULT NULL,
  `indirizzo` varchar(255) DEFAULT NULL,
  `civico` varchar(255) DEFAULT NULL,
  `stato_copertura_indirizzo` varchar(255) DEFAULT NULL,
  `data_pianificazione_prevista` varchar(255) DEFAULT NULL,
  `2_macroaree` varchar(255) DEFAULT NULL,
  `8_macroaree` varchar(255) DEFAULT NULL,
  `data_ultima_variazione_copertura_indirizzo` varchar(255) DEFAULT NULL,
  KEY `id_egon_civico` (`id_egon_civico`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for istat
-- ----------------------------
DROP TABLE IF EXISTS `istat`;
CREATE TABLE `istat` (
  `codice_regione` varchar(2) DEFAULT NULL,
  `codice_provincia` varchar(3) DEFAULT NULL,
  `progressivo_comune` varchar(3) DEFAULT NULL,
  `codice_comune` varchar(6) NOT NULL,
  `nome_comune` varchar(255) DEFAULT NULL,
  `nome_regione` varchar(255) DEFAULT NULL,
  `nome_provincia` varchar(255) DEFAULT NULL,
  `sigla_provincia` varchar(255) DEFAULT NULL,
  `codice_comune_numerico` int(11) DEFAULT NULL,
  `codice_catastale` varchar(4) DEFAULT NULL,
  PRIMARY KEY (`codice_comune`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for log
-- ----------------------------
DROP TABLE IF EXISTS `log`;
CREATE TABLE `log` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `timestamp` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `op` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=75206 DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for netmap_0000953650178cf38c94f42160f0bb42
-- ----------------------------
DROP TABLE IF EXISTS `netmap_0000953650178cf38c94f42160f0bb42`;
CREATE TABLE `netmap_0000953650178cf38c94f42160f0bb42` (
  `PROGRESSIVO_RIGA` int(15) NOT NULL,
  `DR` varchar(50) DEFAULT NULL,
  `CODICE_EGON_CLIENTE` varchar(80) DEFAULT NULL,
  `CODICE_EGON_STRADA_CLIENTE` varchar(80) DEFAULT NULL,
  `CATEGORIA` varchar(2) DEFAULT NULL,
  `COPERTURA_PER_PROSSIMITA` varchar(2) DEFAULT NULL,
  `CODICE_TOP_CLIENTE` varchar(80) DEFAULT NULL,
  `PROVINCIA_CLIENTE` varchar(40) DEFAULT NULL,
  `COMUNE_CLIENTE` varchar(80) DEFAULT NULL,
  `CODICE_ISTAT_COMUNE` varchar(20) DEFAULT NULL,
  `PART_TOP_CLIENTE` varchar(20) DEFAULT NULL,
  `STRADA_CLIENTE` varchar(4000) DEFAULT NULL,
  `CIVICO_CLIENTE` varchar(20) DEFAULT NULL,
  `CODICE_TOP_SITE` varchar(80) DEFAULT NULL,
  `FLAG_OPA` varchar(10) DEFAULT NULL,
  `TOTALE_UI` varchar(10) DEFAULT NULL,
  `BUSINESS` varchar(50) DEFAULT NULL,
  `BANDO_PUBBLICO` varchar(50) DEFAULT NULL,
  `SWITCH_OFF` varchar(5) DEFAULT NULL,
  `SPECIAL_PRICE` varchar(255) DEFAULT NULL,
  `AREA` varchar(50) DEFAULT NULL,
  `COPERTURA_SLU` varchar(20) DEFAULT NULL,
  `VELOCITA_DOWN_SLU` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_SLU` varchar(10) DEFAULT NULL,
  `DISTANZA_SLU` varchar(20) DEFAULT NULL,
  `EDR_SLU` varchar(100) DEFAULT NULL,
  `TIPO_EDR_SLU` varchar(100) DEFAULT NULL,
  `ARMADIO_PADRE_SLU` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_NGA_VULA_FTTCab` varchar(20) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTCab` varchar(20) DEFAULT NULL,
  `VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_BITSTREAM_NGA_VULA_FTTCab` varchar(10) DEFAULT NULL,
  `COPERTURA_EVDSL_BITSTREAM_NGA_VULA_FTTCab` varchar(20) DEFAULT NULL,
  `VELOCITA_DOWN_EVDSL_BITSTREAM_NGA_VULA_FTTCab` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_EVDSL_BITSTREAM_NGA_VULA_FTTCab` varchar(10) DEFAULT NULL,
  `DISTANZA_BITSTREAM_NGA_VULA_FTTCab` varchar(20) DEFAULT NULL,
  `EDR_BITSTREAM_NGA_VULA_FTTCab` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_NGA_VULA_FTTCab` varchar(100) DEFAULT NULL,
  `CLLI_CENTRALE_OLT_BITSTREAM_NGA_VULA_FTTCab` varchar(100) DEFAULT NULL,
  `PREVISIONE_AMPL_BITSTREAM_NGA_VULA_FTTCab` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_NGA_VULA_FTTH` varchar(20) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTH` varchar(20) DEFAULT NULL,
  `COPERTURA_BITSTREAM_NGA_VULA_XGSPON` varchar(100) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_NGA_VULA_XGSPON` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_NGA_VULA_FTTH` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_NGA_VULA_FTTH` varchar(100) DEFAULT NULL,
  `DIRETTO_INFRATEL` varchar(5) DEFAULT NULL,
  `TIPO_COPERTURA_FTTH` varchar(50) DEFAULT NULL,
  `COPERTURA_BITSTREAM_ASIM_ATM` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_ASIM_ATM` varchar(20) DEFAULT NULL,
  `VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_6dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `VELOCITA_DOWN_11dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_11dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `VELOCITA_DOWN_12dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_12dB_BITSTREAM_ASIM_ATM` varchar(10) DEFAULT NULL,
  `DISTANZA_BITSTREAM_ASIM_ATM` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_ASIM_ATM` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_ASIM_ATM` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_ASIM_ETHERNET` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_ASIM_ETH` varchar(20) DEFAULT NULL,
  `VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_6dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `VELOCITA_DOWN_11dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_11dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `VELOCITA_DOWN_12dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_12dB_BITSTREAM_ASIM_ETHERNET` varchar(10) DEFAULT NULL,
  `DISTANZA_BITSTREAM_ASIM_ETHERNET` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_ASIM_ETHERNET` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_ASIM_ETHERNET` varchar(100) DEFAULT NULL,
  `TIPO_DSLAM_BITSTREAM_ASIM_ETHERNET` varchar(32) DEFAULT NULL,
  `PREVISIONE_AMPL_BITSTREAM_ASIM_ETHERNET` varchar(100) DEFAULT NULL,
  `STATO_SERVIZIO_BITSTREAM_ASIM_ETHERNET` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_SIMM_ATM_2M_4M_6M_8M` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_SIMM_ATM_2M_4M_6M_8M` varchar(20) DEFAULT NULL,
  `DISTANZA_BITSTREAM_SIMM_ATM_2M_4M_6M_8M` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_SIMM_ATM_2M_4M_6M_8M` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_SIMM_ATM_2M_4M_6M_8M` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding` varchar(20) DEFAULT NULL,
  `DISTANZA_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding` varchar(100) DEFAULT NULL,
  `COPERTURA_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M` varchar(20) DEFAULT NULL,
  `DISTANZA_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M` varchar(100) DEFAULT NULL,
  `EDR_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M` varchar(100) DEFAULT NULL,
  `TIPO_EDR_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M` varchar(100) DEFAULT NULL,
  `COPERTURA_ULL` varchar(50) DEFAULT NULL,
  `VELOCITA_DOWN_ULL_ADSL2+` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_ULL_ADSL2+` varchar(10) DEFAULT NULL,
  `VELOCITA_DOWN_ULL_VDSL` varchar(10) DEFAULT NULL,
  `VELOCITA_UP_ULL_VDSL` varchar(10) DEFAULT NULL,
  `DISTANZA_ULL` varchar(100) DEFAULT NULL,
  `EDR_ULL` varchar(100) DEFAULT NULL,
  `TIPO_EDR_ULL` varchar(100) DEFAULT NULL,
  `COPERTURA_WLR` varchar(50) DEFAULT NULL,
  `DISTANZA_WLR` varchar(100) DEFAULT NULL,
  `EDR_WLR` varchar(100) DEFAULT NULL,
  `TIPO_EDR_WLR` varchar(100) DEFAULT NULL,
  `COPERTURA_CIRCUITI_TERM_ETH_FIBRA_OTTICA` varchar(50) DEFAULT NULL,
  `PIANIFICAZIONE_CIRCUITI_TERM_ETH_FIBRA_OTTICA` varchar(20) DEFAULT NULL,
  `FASCIA_CIRCUITI_TERM_ETH_FIBRA_OTTICA` varchar(100) DEFAULT NULL,
  `EDR_CIRCUITI_TERM_ETH_FIBRA_OTTICA` varchar(100) DEFAULT NULL,
  `TIPO_EDR_CIRCUITI_TERM_ETH_FIBRA_OTTICA` varchar(100) DEFAULT NULL,
  PRIMARY KEY (`PROGRESSIVO_RIGA`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1 ROW_FORMAT=COMPACT;

-- ----------------------------
-- Table structure for openfiber
-- ----------------------------
DROP TABLE IF EXISTS `openfiber`;
CREATE TABLE `openfiber` (
  `ID_SCALA` varchar(160) NOT NULL,
  `REGIONE` varchar(30) DEFAULT NULL,
  `PROVINCIA` varchar(30) DEFAULT NULL,
  `COMUNE` varchar(60) DEFAULT NULL,
  `FRAZIONE` varchar(20) DEFAULT NULL,
  `PARTICELLA_TOP` varchar(28) DEFAULT NULL,
  `INDIRIZZO` varchar(70) DEFAULT NULL,
  `CIVICO` varchar(10) DEFAULT NULL,
  `SCALA_PALAZZINA` varchar(10) DEFAULT NULL,
  `CODICE_VIA` varchar(10) DEFAULT NULL,
  `ID_BUILDING` varchar(150) DEFAULT NULL,
  `COORDINATE_BUILDING` varchar(40) DEFAULT NULL,
  `POP` varchar(10) DEFAULT NULL,
  `TOTALE_UI` varchar(4) DEFAULT NULL,
  `STATO_BUILDING` varchar(3) DEFAULT NULL,
  `STATO_SCALA_PALAZZINA` varchar(3) DEFAULT NULL,
  `DATA_RFC_INDICATIVA` varchar(10) DEFAULT NULL,
  `DATA_RFC_EFFETTIVA` varchar(10) DEFAULT NULL,
  `DATA_RFA_INDICATIVA` varchar(10) DEFAULT NULL,
  `DATA_RFA_EFFETTIVA` varchar(10) DEFAULT NULL,
  `DATA_ULTIMA_MODIFICA_RECORD` varchar(10) DEFAULT NULL,
  `DATA_ULTIMA_VARIAZIONE_STATO_BUILDING` varchar(10) DEFAULT NULL,
  `DATA_ULTIMA_VARIAZIONE_STATO_SCALA_PALAZZINA` varchar(10) DEFAULT NULL,
  `ID_EGON_CIVICO` varchar(15) NOT NULL,
  `ID_EGON_STRADA` varchar(11) DEFAULT NULL,
  `CLUSTER` varchar(3) DEFAULT NULL,
  PRIMARY KEY (`ID_SCALA`,`ID_EGON_CIVICO`) USING BTREE,
  KEY `ID_EGON_CIVICO` (`ID_EGON_CIVICO`) USING BTREE,
  KEY `ID_EGON_STRADA` (`ID_EGON_STRADA`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for openfiber_status
-- ----------------------------
DROP TABLE IF EXISTS `openfiber_status`;
CREATE TABLE `openfiber_status` (
  `value` int(11) NOT NULL,
  `description` varchar(255) DEFAULT NULL,
  `cluster` varchar(3) NOT NULL,
  `friendly_description` varchar(255) DEFAULT NULL,
  PRIMARY KEY (`value`,`cluster`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for openfiber_xgpon_pop
-- ----------------------------
DROP TABLE IF EXISTS `openfiber_xgpon_pop`;
CREATE TABLE `openfiber_xgpon_pop` (
  `pop` varchar(255) NOT NULL,
  PRIMARY KEY (`pop`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for pcn_pops
-- ----------------------------
DROP TABLE IF EXISTS `pcn_pops`;
CREATE TABLE `pcn_pops` (
  `pop` varchar(10) NOT NULL,
  PRIMARY KEY (`pop`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for premium_verify_log
-- ----------------------------
DROP TABLE IF EXISTS `premium_verify_log`;
CREATE TABLE `premium_verify_log` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `user_id` int(11) NOT NULL,
  `username` varchar(255) NOT NULL,
  `request_date` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `request_json` longtext NOT NULL,
  `request_hash` varchar(255) NOT NULL,
  `answer_json` longtext NOT NULL,
  `answer_hash` varchar(255) NOT NULL,
  `update_time` datetime DEFAULT NULL,
  PRIMARY KEY (`id`) USING BTREE
) ENGINE=InnoDB AUTO_INCREMENT=681 DEFAULT CHARSET=latin1 ROW_FORMAT=COMPACT;

-- ----------------------------
-- Table structure for radio_table
-- ----------------------------
DROP TABLE IF EXISTS `radio_table`;
CREATE TABLE `radio_table` (
  `id_ponte` int(11) NOT NULL AUTO_INCREMENT,
  `nome` varchar(255) DEFAULT NULL,
  `lat` varchar(255) DEFAULT NULL,
  `lon` varchar(255) DEFAULT NULL,
  `alt` varchar(255) DEFAULT NULL,
  `stato` varchar(255) DEFAULT NULL,
  `descrizione` varchar(255) DEFAULT NULL,
  `fascia` varchar(1) NOT NULL DEFAULT 'D',
  PRIMARY KEY (`id_ponte`)
) ENGINE=InnoDB AUTO_INCREMENT=114 DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for requests_log
-- ----------------------------
DROP TABLE IF EXISTS `requests_log`;
CREATE TABLE `requests_log` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `request_date` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP,
  `user` varchar(255) NOT NULL,
  `request_json` longtext NOT NULL,
  `answer_json` longtext NOT NULL,
  `update_time` datetime DEFAULT NULL,
  PRIMARY KEY (`id`)
) ENGINE=InnoDB AUTO_INCREMENT=51502 DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for sgu_fw
-- ----------------------------
DROP TABLE IF EXISTS `sgu_fw`;
CREATE TABLE `sgu_fw` (
  `gat` varchar(255) DEFAULT NULL,
  `clli` varchar(255) DEFAULT NULL,
  `descrizione_gat` varchar(255) DEFAULT NULL,
  `dslam` varchar(255) DEFAULT NULL,
  `minidslam` varchar(255) DEFAULT NULL,
  `citta` varchar(255) DEFAULT NULL,
  `provincia` varchar(255) DEFAULT NULL,
  `regione` varchar(255) DEFAULT NULL,
  `2_macroaree` varchar(255) DEFAULT NULL,
  `8_macroaree` varchar(255) DEFAULT NULL,
  `stato_copertura` varchar(255) DEFAULT NULL,
  `data_pianificazione` varchar(255) DEFAULT NULL
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for sirius_egon_polylines
-- ----------------------------
DROP TABLE IF EXISTS `sirius_egon_polylines`;
CREATE TABLE `sirius_egon_polylines` (
  `egon_strada` varchar(255) NOT NULL,
  `egon_comune` varchar(255) NOT NULL,
  `strada` varchar(255) NOT NULL,
  `polylines` text,
  PRIMARY KEY (`egon_strada`,`egon_comune`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for sirius_netmap
-- ----------------------------
DROP TABLE IF EXISTS `sirius_netmap`;
CREATE TABLE `sirius_netmap` (
  `egon_civico` varchar(255) NOT NULL,
  `egon_strada` varchar(255) NOT NULL,
  `egon_comune` varchar(255) NOT NULL,
  `egon_provincia` varchar(255) NOT NULL,
  `egon_regione` varchar(255) NOT NULL,
  `civico` varchar(255) NOT NULL,
  `strada` varchar(255) NOT NULL,
  `comune` varchar(255) NOT NULL,
  `provincia` varchar(255) NOT NULL,
  `regione` varchar(255) NOT NULL,
  `id_tipo_servizio` int(11) NOT NULL,
  `id_stato` int(11) NOT NULL,
  `data_stato_corrente` datetime NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  `note` text,
  PRIMARY KEY (`egon_civico`,`egon_strada`,`egon_comune`,`egon_provincia`,`egon_regione`,`id_tipo_servizio`) USING BTREE,
  KEY `id_tipo_servizio` (`id_tipo_servizio`),
  KEY `id_stato` (`id_stato`)
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for sirius_stati
-- ----------------------------
DROP TABLE IF EXISTS `sirius_stati`;
CREATE TABLE `sirius_stati` (
  `id_stato` int(11) NOT NULL AUTO_INCREMENT,
  `name` varchar(255) NOT NULL,
  PRIMARY KEY (`id_stato`)
) ENGINE=InnoDB AUTO_INCREMENT=7 DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for sirius_tipi_servizio
-- ----------------------------
DROP TABLE IF EXISTS `sirius_tipi_servizio`;
CREATE TABLE `sirius_tipi_servizio` (
  `id_tipo_servizio` int(11) NOT NULL AUTO_INCREMENT,
  `name` varchar(255) NOT NULL,
  PRIMARY KEY (`id_tipo_servizio`)
) ENGINE=InnoDB AUTO_INCREMENT=7 DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for tab_vlan
-- ----------------------------
DROP TABLE IF EXISTS `tab_vlan`;
CREATE TABLE `tab_vlan` (
  `id_vlan` int(11) NOT NULL,
  `id_ordine_olo` varchar(255) DEFAULT NULL,
  `centrale` varchar(255) DEFAULT NULL,
  `adr` varchar(255) DEFAULT NULL,
  `clli` varchar(255) DEFAULT NULL,
  `tipo` varchar(255) DEFAULT NULL,
  `stato` tinyint(4) DEFAULT NULL,
  PRIMARY KEY (`id_vlan`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=utf8 ROW_FORMAT=COMPACT;

-- ----------------------------
-- Table structure for vula_clli
-- ----------------------------
DROP TABLE IF EXISTS `vula_clli`;
CREATE TABLE `vula_clli` (
  `clli` varchar(255) NOT NULL,
  PRIMARY KEY (`clli`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

-- ----------------------------
-- Table structure for vula_fw
-- ----------------------------
DROP TABLE IF EXISTS `vula_fw`;
CREATE TABLE `vula_fw` (
  `id_record` int(20) DEFAULT NULL,
  `id_egon_civico` varchar(255) DEFAULT NULL,
  `particella_top_egon` varchar(255) DEFAULT NULL,
  `nome_via_egon` varchar(255) DEFAULT NULL,
  `civico_top_egon` varchar(255) DEFAULT NULL,
  `id_egon_via` varchar(255) DEFAULT NULL,
  `cap_comune` varchar(255) DEFAULT NULL,
  `comune_egon` varchar(255) DEFAULT NULL,
  `provincia_egon` varchar(255) DEFAULT NULL,
  `sigla_provincia_egon` varchar(255) DEFAULT NULL,
  `regione` varchar(255) DEFAULT NULL,
  `stato_vendibilita_vula` varchar(255) DEFAULT NULL,
  `stato_tecnico_vula` varchar(255) DEFAULT NULL,
  `previsione_ampliamento_vula` varchar(255) DEFAULT NULL,
  `flag_opa` varchar(255) DEFAULT NULL,
  `id_sgu_centrale` varchar(255) DEFAULT NULL,
  `nome_centrale` varchar(255) DEFAULT NULL,
  `id_cabinet` varchar(255) DEFAULT NULL,
  `tipo_vula` varchar(255) DEFAULT NULL,
  `distanza_cabinet_vula` varchar(255) DEFAULT NULL,
  `categoria` varchar(255) DEFAULT NULL,
  `velocita_down_vula` varchar(255) DEFAULT NULL,
  `velocita_up_vula` varchar(255) DEFAULT NULL,
  KEY `id_egon_civico` (`id_egon_civico`) USING BTREE
) ENGINE=InnoDB DEFAULT CHARSET=latin1;

SET FOREIGN_KEY_CHECKS = 1;
