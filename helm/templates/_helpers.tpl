{{- define "github-actions-exporter.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "github-actions-exporter.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{- define "github-actions-exporter.namespace" -}}
{{- default .Release.Namespace .Values.namespaceOverride }}
{{- end }}

{{- define "github-actions-exporter.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{- define "github-actions-exporter.imageTag" -}}
{{- .Values.image.tag | default .Chart.AppVersion }}
{{- end }}

{{- define "github-actions-exporter.image" -}}
{{- $repository := required "image.repository is required" .Values.image.repository -}}
{{- with .Values.image.registry -}}
{{- $repository = printf "%s/%s" . $repository -}}
{{- end -}}
{{- with .Values.image.digest -}}
{{- printf "%s@%s" $repository . -}}
{{- else -}}
{{- printf "%s:%s" $repository (include "github-actions-exporter.imageTag" $) -}}
{{- end -}}
{{- end }}

{{- define "github-actions-exporter.labels" -}}
{{- $labels := dict
      "helm.sh/chart" (include "github-actions-exporter.chart" .)
      "app.kubernetes.io/name" (include "github-actions-exporter.name" .)
      "app.kubernetes.io/instance" .Release.Name
      "app.kubernetes.io/component" "exporter"
      "app.kubernetes.io/part-of" (include "github-actions-exporter.name" .)
      "app.kubernetes.io/managed-by" .Release.Service -}}
{{- with (include "github-actions-exporter.imageTag" .) -}}
{{- $_ := set $labels "app.kubernetes.io/version" . -}}
{{- end -}}
{{- if .Values.releaseLabel -}}
{{- $_ := set $labels "release" .Release.Name -}}
{{- end -}}
{{- $common := .Values.commonLabels | default dict -}}
{{- toYaml (merge $labels (deepCopy $common)) -}}
{{- end }}

{{- define "github-actions-exporter.labelsWith" -}}
{{- $base := fromYaml (include "github-actions-exporter.labels" .context) -}}
{{- $extra := .extra | default dict -}}
{{- toYaml (merge (deepCopy $extra) $base) -}}
{{- end }}

{{- define "github-actions-exporter.selectorLabels" -}}
app.kubernetes.io/name: {{ include "github-actions-exporter.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "github-actions-exporter.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- .Values.serviceAccount.name | default (include "github-actions-exporter.fullname" .) }}
{{- else }}
{{- .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{- define "github-actions-exporter.secretName" -}}
{{- .Values.webhookSecret.existingSecret | default (include "github-actions-exporter.fullname" .) }}
{{- end }}
