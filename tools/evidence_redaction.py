"""Keep device descriptors/calibration/control values out of public replay evidence."""
def public_replay_summary(summary):
    result = {key: value for key, value in summary.items() if key != 'context'}
    context = summary.get('context')
    controls = context.get('controls') if isinstance(context, dict) else None
    result['capture_context_present'] = isinstance(context, dict)
    result['capture_observations_present'] = isinstance(controls, dict) and 'capture_observations' in controls
    result['capture_context_redacted'] = True
    return result
