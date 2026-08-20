//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/config_diagnostic.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_validation_response.g.dart';

/// ApplicationConfigValidationResponse
///
/// Properties:
/// * [diagnostics]
/// * [valid]
@BuiltValue()
abstract class ApplicationConfigValidationResponse implements Built<ApplicationConfigValidationResponse, ApplicationConfigValidationResponseBuilder> {
  @BuiltValueField(wireName: r'diagnostics')
  BuiltList<ConfigDiagnostic> get diagnostics;

  @BuiltValueField(wireName: r'valid')
  bool get valid;

  ApplicationConfigValidationResponse._();

  factory ApplicationConfigValidationResponse([void updates(ApplicationConfigValidationResponseBuilder b)]) = _$ApplicationConfigValidationResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigValidationResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigValidationResponse> get serializer => _$ApplicationConfigValidationResponseSerializer();
}

class _$ApplicationConfigValidationResponseSerializer implements PrimitiveSerializer<ApplicationConfigValidationResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigValidationResponse, _$ApplicationConfigValidationResponse];

  @override
  final String wireName = r'ApplicationConfigValidationResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigValidationResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'diagnostics';
    yield serializers.serialize(
      object.diagnostics,
      specifiedType: const FullType(BuiltList, [FullType(ConfigDiagnostic)]),
    );
    yield r'valid';
    yield serializers.serialize(
      object.valid,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigValidationResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigValidationResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'diagnostics':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ConfigDiagnostic)]),
          ) as BuiltList<ConfigDiagnostic>;
          result.diagnostics.replace(valueDes);
          break;
        case r'valid':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.valid = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationConfigValidationResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigValidationResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
